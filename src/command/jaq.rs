use std::{
    convert::Infallible,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use anyhow::anyhow;
use jaq_all::{
    data::{self, Runner},
    fmts::read,
    jaq_core::Vars,
    json::{Val, ValX},
    load::FileReportsDisp,
};
use tokio::sync::mpsc;

use crate::input_format::InputFormat;

const OUTPUT_BUFFER_CAPACITY: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Jaq {
    pub(crate) query: String,
}

pub(crate) fn parse(arguments: &str) -> anyhow::Result<Jaq> {
    if arguments.is_empty() {
        Err(anyhow::anyhow!("jaq requires a query"))
    } else {
        Ok(Jaq {
            query: arguments.to_owned(),
        })
    }
}

/// Identifies one invocation of a jaq query.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct QueryId(u64);

/// Describes how a jaq query stopped.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Completion {
    Completed,
    Cancelled,
    Failed(String),
}

/// A value or completion notification produced by the currently active query.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Event {
    Output {
        query_id: QueryId,
        value: Val,
    },
    Finished {
        query_id: QueryId,
        completion: Completion,
    },
}

impl Event {
    fn query_id(&self) -> QueryId {
        match self {
            Self::Output { query_id, .. } | Self::Finished { query_id, .. } => *query_id,
        }
    }
}

/// Parses a document into the values supplied to jaq queries.
pub(crate) fn parse_inputs(format: InputFormat, input: &[u8]) -> anyhow::Result<Vec<Val>> {
    match format {
        InputFormat::Json => read::json::parse_many(input)
            .map(|result| result.map_err(|error| anyhow!(error.to_string())))
            .collect(),
        InputFormat::Yaml => {
            let input = std::str::from_utf8(input)?;
            read::yaml::parse_many(input)
                .map(|result| result.map_err(|error| anyhow!(error.to_string())))
                .collect()
        }
        InputFormat::Auto => unreachable!("input format must be resolved before parsing"),
    }
}

struct ActiveQuery {
    id: QueryId,
    cancelled: Arc<AtomicBool>,
}

/// Runs jaq queries on Tokio's blocking thread pool and streams their outputs.
///
/// Starting a new query requests cancellation of the previous query. Cancellation is cooperative:
/// it is observed when jaq produces its next output. A filter that never yields an output cannot be
/// forcibly stopped by this executor.
pub(crate) struct JaqExecutor {
    next_query_id: u64,
    active: Option<ActiveQuery>,
    event_tx: mpsc::Sender<Event>,
    event_rx: mpsc::Receiver<Event>,
}

impl Default for JaqExecutor {
    fn default() -> Self {
        let (event_tx, event_rx) = mpsc::channel(OUTPUT_BUFFER_CAPACITY);
        Self {
            next_query_id: 0,
            active: None,
            event_tx,
            event_rx,
        }
    }
}

impl JaqExecutor {
    /// Starts a query without waiting for it to finish.
    ///
    /// Outputs from queries superseded by a newer invocation are discarded by [`Self::recv`].
    pub(crate) fn execute(
        &mut self,
        query: impl Into<String>,
        inputs: impl Into<Arc<[Val]>>,
    ) -> QueryId {
        self.cancel();

        self.next_query_id = self.next_query_id.wrapping_add(1);
        let id = QueryId(self.next_query_id);
        let cancelled = Arc::new(AtomicBool::new(false));
        self.active = Some(ActiveQuery {
            id,
            cancelled: cancelled.clone(),
        });

        let event_tx = self.event_tx.clone();
        let query = query.into();
        let inputs = inputs.into();
        tokio::task::spawn_blocking(move || {
            let completion = run_query(id, &query, &inputs, &cancelled, &event_tx);
            let _ = event_tx.blocking_send(Event::Finished {
                query_id: id,
                completion,
            });
        });

        id
    }

    /// Requests cancellation of the active query.
    ///
    /// Returns `true` if a query was active. The corresponding [`Event::Finished`] is emitted once
    /// jaq reaches the next output boundary and observes the request.
    pub(crate) fn cancel(&self) -> bool {
        let Some(active) = &self.active else {
            return false;
        };

        active.cancelled.store(true, Ordering::Release);
        true
    }

    /// Receives the next output or completion event for the active query.
    ///
    /// Events buffered by superseded queries are drained and discarded.
    #[allow(
        dead_code,
        reason = "the UI batches buffered events with try_recv, while other callers may await recv"
    )]
    pub(crate) async fn recv(&mut self) -> Option<Event> {
        while let Some(event) = self.event_rx.recv().await {
            if let Some(event) = self.accept_active_event(event) {
                return Some(event);
            }
        }

        None
    }

    /// Receives an already-buffered event without waiting.
    pub(crate) fn try_recv(&mut self) -> Option<Event> {
        while let Ok(event) = self.event_rx.try_recv() {
            if let Some(event) = self.accept_active_event(event) {
                return Some(event);
            }
        }

        None
    }

    fn accept_active_event(&mut self, event: Event) -> Option<Event> {
        let active = self.active.as_ref()?;
        if event.query_id() != active.id {
            return None;
        }
        if matches!(event, Event::Output { .. }) && active.cancelled.load(Ordering::Acquire) {
            return None;
        }

        if matches!(event, Event::Finished { .. }) {
            self.active = None;
        }
        Some(event)
    }
}

impl Drop for JaqExecutor {
    fn drop(&mut self) {
        self.cancel();
    }
}

#[derive(Debug)]
enum RunError {
    Cancelled,
    Failed(anyhow::Error),
}

fn run_query(
    id: QueryId,
    query: &str,
    inputs: &[Val],
    cancelled: &AtomicBool,
    event_tx: &mpsc::Sender<Event>,
) -> Completion {
    if cancelled.load(Ordering::Acquire) {
        return Completion::Cancelled;
    }

    let filter = match data::compile(query) {
        Ok(filter) => filter,
        Err(reports) => {
            let message = reports
                .iter()
                .map(|report| FileReportsDisp::new(report).to_string())
                .collect();
            return Completion::Failed(message);
        }
    };
    let runner = Runner::default();

    let result = data::run(
        &runner,
        &filter,
        Vars::default(),
        inputs.iter().cloned().map(Ok::<Val, Infallible>),
        |error| RunError::Failed(anyhow!(error)),
        |result| {
            if cancelled.load(Ordering::Acquire) {
                return Err(RunError::Cancelled);
            }

            let value = result_value(result)?;
            event_tx
                .blocking_send(Event::Output {
                    query_id: id,
                    value,
                })
                .map_err(|_| RunError::Failed(anyhow!("jaq event receiver was dropped")))
        },
    );

    match result {
        Ok(()) if cancelled.load(Ordering::Acquire) => Completion::Cancelled,
        Ok(()) => Completion::Completed,
        Err(RunError::Cancelled) => Completion::Cancelled,
        Err(RunError::Failed(error)) => Completion::Failed(error.to_string()),
    }
}

fn result_value(result: ValX<'_>) -> Result<Val, RunError> {
    match result {
        Ok(value) => Ok(value),
        Err(exception) => match exception.get_err() {
            Ok(error) => Err(RunError::Failed(anyhow!(error.to_string()))),
            Err(exception) => match exception.get_halt() {
                Ok(code) => Err(RunError::Failed(anyhow!(
                    "jaq halted with exit code {code}"
                ))),
                Err(exception) => Err(RunError::Failed(anyhow!(
                    "unexpected jaq exception: {exception:?}"
                ))),
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use jaq_all::json::read::parse_single;
    use tokio::time::timeout;

    use super::*;

    #[test]
    fn parses_a_query() {
        assert_eq!(
            parse(".items[]").unwrap(),
            Jaq {
                query: ".items[]".into()
            }
        );
    }

    #[test]
    fn rejects_an_empty_query() {
        assert!(parse("").is_err());
    }

    fn json(source: &str) -> Val {
        parse_single(source.as_bytes()).expect("valid JSON")
    }

    #[test]
    fn parses_json_and_yaml_inputs() {
        assert_eq!(
            parse_inputs(InputFormat::Json, b"{\"json\":true}\n42").unwrap(),
            [json(r#"{"json":true}"#), json("42")]
        );
        assert_eq!(
            parse_inputs(InputFormat::Yaml, b"---\nyaml: true\n---\n42\n").unwrap(),
            [json(r#"{"yaml":true}"#), json("42")]
        );
    }

    async fn next_event(jaq: &mut JaqExecutor) -> Event {
        timeout(Duration::from_secs(2), jaq.recv())
            .await
            .expect("jaq event timeout")
            .expect("jaq event channel closed")
    }

    #[tokio::test]
    async fn streams_outputs_before_the_query_finishes() {
        let mut jaq = JaqExecutor::default();
        let query_id = jaq.execute("range(0; 3)", vec![json("null")]);

        for expected in 0..3 {
            assert_eq!(
                next_event(&mut jaq).await,
                Event::Output {
                    query_id,
                    value: json(&expected.to_string()),
                }
            );
        }
        assert_eq!(
            next_event(&mut jaq).await,
            Event::Finished {
                query_id,
                completion: Completion::Completed,
            }
        );
    }

    #[tokio::test]
    async fn replaces_an_infinite_output_query_without_waiting_for_it() {
        let mut jaq = JaqExecutor::default();
        let first_id = jaq.execute("0 | recurse(. + 1)", vec![json("null")]);
        assert_eq!(
            next_event(&mut jaq).await,
            Event::Output {
                query_id: first_id,
                value: json("0"),
            }
        );

        let second_id = jaq.execute(".", vec![json("42")]);

        assert_eq!(
            next_event(&mut jaq).await,
            Event::Output {
                query_id: second_id,
                value: json("42"),
            }
        );
        assert_eq!(
            next_event(&mut jaq).await,
            Event::Finished {
                query_id: second_id,
                completion: Completion::Completed,
            }
        );
    }

    #[tokio::test]
    async fn cancels_an_infinite_output_query() {
        let mut jaq = JaqExecutor::default();
        let query_id = jaq.execute("0 | recurse(. + 1)", vec![json("null")]);
        let _ = next_event(&mut jaq).await;

        assert!(jaq.cancel());
        assert_eq!(
            next_event(&mut jaq).await,
            Event::Finished {
                query_id,
                completion: Completion::Cancelled,
            }
        );
    }

    #[tokio::test]
    async fn reports_query_errors() {
        let mut jaq = JaqExecutor::default();
        let query_id = jaq.execute(".【", vec![json("null")]);

        let Event::Finished {
            query_id: actual_id,
            completion: Completion::Failed(message),
        } = next_event(&mut jaq).await
        else {
            panic!("expected a failed completion event");
        };
        assert_eq!(actual_id, query_id);
        assert!(!message.is_empty());
    }
}
