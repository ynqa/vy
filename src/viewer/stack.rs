use super::{ConfigUpdate, View, ViewNotification};

/// Child views return to their caller. The root view remains until the application exits.
pub(in crate::viewer) struct ViewStack {
    views: Vec<Box<dyn View>>,
}

impl ViewStack {
    pub(in crate::viewer) fn new(root: Box<dyn View>) -> Self {
        Self { views: vec![root] }
    }

    pub(in crate::viewer) fn active(&self) -> &dyn View {
        self.views
            .last()
            .expect("the root view remains on the stack")
            .as_ref()
    }

    pub(in crate::viewer) fn active_mut(&mut self) -> &mut dyn View {
        self.views
            .last_mut()
            .expect("the root view remains on the stack")
            .as_mut()
    }

    pub(in crate::viewer) fn push(&mut self, view: Box<dyn View>) {
        self.views.push(view);
    }

    pub(in crate::viewer) fn back(&mut self) {
        if self.views.len() > 1 {
            self.views.pop();
        }
    }

    /// Configuration is shared by active and suspended views, including nested focus views.
    pub(in crate::viewer) fn update_config(&mut self, update: ConfigUpdate) {
        let update = std::sync::Arc::new(update);
        for view in &mut self.views {
            view.notify(ViewNotification::ConfigUpdated(update.clone()));
        }
    }

    pub(in crate::viewer) fn return_to_caller(&mut self, notification: ViewNotification) {
        self.back();
        self.active_mut().notify(notification);
    }
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use super::*;
    use crate::viewer::{ViewContext, ViewEvent, ViewFrame, ViewUpdate};

    #[derive(Default)]
    struct Received {
        path: Option<(usize, String)>,
        command: Option<String>,
        config: Option<std::sync::Arc<ConfigUpdate>>,
    }

    struct Host(Rc<RefCell<Received>>);

    impl View for Host {
        fn context(&self) -> ViewContext {
            ViewContext::Help
        }
        fn render(&mut self, _: u16, _: u16) -> ViewFrame {
            ViewFrame::default()
        }
        fn notify(&mut self, notification: ViewNotification) {
            match notification {
                ViewNotification::Navigate {
                    document_index,
                    path,
                } => self.0.borrow_mut().path = Some((document_index, path)),
                ViewNotification::RecallCommand(command) => {
                    self.0.borrow_mut().command = Some(command)
                }
                ViewNotification::ConfigUpdated(update) => {
                    self.0.borrow_mut().config = Some(update)
                }
                _ => {}
            }
        }
        fn update(&mut self, _event: ViewEvent<'_>) -> ViewUpdate {
            ViewUpdate::Ignored
        }
    }

    #[test]
    fn broadcasts_configuration_to_active_and_suspended_views() {
        let root = Rc::new(RefCell::new(Received::default()));
        let child = Rc::new(RefCell::new(Received::default()));
        let mut stack = ViewStack::new(Box::new(Host(root.clone())));
        stack.push(Box::new(Host(child.clone())));
        let mut config = crate::config::Config::default();
        config.json.indent = 7;

        stack.update_config(ConfigUpdate {
            message: "configuration reloaded".into(),
            config,
        });

        let root_config = root.borrow().config.clone().unwrap();
        let child_config = child.borrow().config.clone().unwrap();
        assert!(std::sync::Arc::ptr_eq(&root_config, &child_config));
        assert_eq!(root_config.config.json.indent, 7);
        assert_eq!(stack.views.len(), 2);
    }

    #[test]
    fn returns_navigation_and_history_to_the_immediate_caller() {
        let root = Rc::new(RefCell::new(Received::default()));
        let caller = Rc::new(RefCell::new(Received::default()));
        let mut stack = ViewStack::new(Box::new(Host(root.clone())));
        stack.push(Box::new(Host(caller.clone())));
        stack.push(Box::new(Host(Rc::default())));

        stack.return_to_caller(ViewNotification::Navigate {
            document_index: 0,
            path: ".local".into(),
        });
        assert_eq!(stack.views.len(), 2);
        assert_eq!(caller.borrow().path, Some((0, ".local".into())));
        assert!(root.borrow().path.is_none());

        stack.push(Box::new(Host(Rc::default())));
        stack.return_to_caller(ViewNotification::RecallCommand("jaq .local".into()));
        assert_eq!(stack.views.len(), 2);
        assert_eq!(caller.borrow().command.as_deref(), Some("jaq .local"));
        assert!(root.borrow().command.is_none());
        // Returning from the caller preserves the original root; repeated back cannot remove it.
        stack.back();
        stack.back();
        assert_eq!(stack.views.len(), 1);
    }
}
