use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tokio::{sync::watch, task};

use crate::document_source::DocumentSource;

use super::{Catalog, Indexer};

/// Observable state of the one-shot background catalog build.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CatalogLoadState {
    Loading,
    Ready(Arc<Catalog>),
    Failed(Arc<str>),
}

/// The receiving side of a catalog load.
///
/// Clones subscribe to the same watch channel and share the completed
/// `Arc<Catalog>`. When every subscription is dropped, the sender is notified
/// and asks the blocking indexer to stop at its next cooperative checkpoint.
#[derive(Clone)]
pub(crate) struct CatalogSubscription {
    state: watch::Receiver<CatalogLoadState>,
}

impl CatalogSubscription {
    #[allow(dead_code, reason = "part of the catalog load subscription API")]
    pub(crate) fn state(&self) -> CatalogLoadState {
        self.state.borrow().clone()
    }

    pub(crate) fn state_and_mark_seen(&mut self) -> CatalogLoadState {
        self.state.borrow_and_update().clone()
    }

    pub(crate) fn has_changed(&self) -> bool {
        self.state.has_changed().unwrap_or(false)
    }

    pub(crate) async fn changed(&mut self) -> Option<CatalogLoadState> {
        self.state.changed().await.ok()?;
        Some(self.state_and_mark_seen())
    }

    #[cfg(test)]
    pub(crate) fn from_state(state: CatalogLoadState) -> Self {
        let (_sender, state) = watch::channel(state);
        Self { state }
    }
}

/// Starts and owns no catalog data itself; the returned subscription is the
/// lifetime handle for the background operation.
pub(crate) struct CatalogLoader;

impl CatalogLoader {
    pub(crate) fn spawn(source: DocumentSource) -> CatalogSubscription {
        let (state_tx, state) = watch::channel(CatalogLoadState::Loading);
        let cancelled = Arc::new(AtomicBool::new(false));
        let task_cancelled = Arc::clone(&cancelled);

        tokio::spawn(async move {
            let cancellation_guard = CancellationGuard(cancelled);
            let indexing_task = task::spawn_blocking(move || {
                Indexer::new(source)
                    .with_cancellation(task_cancelled)
                    .index()
            });
            tokio::pin!(indexing_task);

            tokio::select! {
                result = &mut indexing_task => {
                    let next_state = match result {
                        Ok(Ok(catalog)) => CatalogLoadState::Ready(Arc::new(catalog)),
                        Ok(Err(error)) => CatalogLoadState::Failed(error.to_string().into()),
                        Err(error) => CatalogLoadState::Failed(error.to_string().into()),
                    };
                    state_tx.send_replace(next_state);
                }
                _ = state_tx.closed() => {
                    cancellation_guard.cancel();
                    // A running `spawn_blocking` task cannot be aborted. Waiting here lets
                    // the indexer's cooperative cancellation finish without detaching work.
                    let _ = indexing_task.await;
                }
            }
        });

        CatalogSubscription { state }
    }
}

struct CancellationGuard(Arc<AtomicBool>);

impl CancellationGuard {
    fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }
}

impl Drop for CancellationGuard {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::input_format::InputFormat;

    use super::*;

    fn source(input: &str) -> DocumentSource {
        DocumentSource::from_reader(
            InputFormat::Json,
            Box::new(Cursor::new(input.as_bytes().to_vec())),
        )
        .unwrap()
    }

    #[tokio::test]
    async fn background_load_transitions_once_to_ready() {
        let mut subscription = CatalogLoader::spawn(source(r#"{"name":"Alice"}"#));

        assert_eq!(subscription.state(), CatalogLoadState::Loading);
        let state = subscription.changed().await.expect("indexer stopped");
        let CatalogLoadState::Ready(catalog) = state else {
            panic!("catalog must become ready");
        };
        assert_eq!(catalog.documents().len(), 1);
        assert_eq!(subscription.changed().await, None);
    }

    #[tokio::test]
    async fn background_load_transitions_once_to_failed() {
        let mut subscription = CatalogLoader::spawn(source("{"));

        let state = subscription.changed().await.expect("indexer stopped");
        let CatalogLoadState::Failed(error) = state else {
            panic!("catalog must fail");
        };
        assert!(error.contains("failed to parse JSON for catalog"));
        assert_eq!(subscription.changed().await, None);
    }

    #[tokio::test]
    async fn subscriptions_share_the_same_completed_catalog() {
        let mut first = CatalogLoader::spawn(source(r#"{"name":"Alice"}"#));
        let second = first.clone();

        let CatalogLoadState::Ready(first_catalog) =
            first.changed().await.expect("indexer stopped")
        else {
            panic!("catalog must become ready");
        };
        let CatalogLoadState::Ready(second_catalog) = second.state() else {
            panic!("catalog must become ready for every subscriber");
        };

        assert!(Arc::ptr_eq(&first_catalog, &second_catalog));
    }
}
