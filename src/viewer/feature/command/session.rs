use std::borrow::Cow;

use promkit::core::{CreatedGraphemes, Widget};

use crate::{
    catalog::FlattenSubscription,
    command::{
        Command,
        argument::document_path::{DocumentPath, PathCompleter},
        copy::Target as CopyTarget,
    },
    config::ThemeConfig,
    document_source::DocumentSource,
    history::History,
    viewer::{
        Action, CommandAction, ComponentAction, ViewEffect, ViewEvent, ViewNotification,
        ViewRequest, ViewUpdate,
        feature::{Feedback, FeedbackKind},
        ui::{DocumentComponent, DocumentDisplayConfig},
    },
};

use super::{CommandComponent, CommandOutcome};

/// The document and indexes belonging to the calling view. Paths and document numbers are
/// interpreted within this target, including completion supplied when creating the session.
pub(in crate::viewer) struct CommandTarget<'a> {
    pub(in crate::viewer) document: &'a mut DocumentComponent,
    pub(in crate::viewer) source: &'a DocumentSource,
    pub(in crate::viewer) flatten: &'a FlattenSubscription,
    pub(in crate::viewer) display_config: &'a DocumentDisplayConfig,
}

impl CommandTarget<'_> {
    fn subtree(
        &self,
        location: Option<DocumentPath>,
        command: &str,
    ) -> anyhow::Result<(DocumentSource, DocumentPath)> {
        let location = self.resolve_path(location)?;
        let source = self
            .source
            .subtree(location.document_index, &location.path)
            .map_err(|error| {
                anyhow::anyhow!(
                    "cannot {command} @{} {}: {error}",
                    location.document_index,
                    location.path
                )
            })?;
        Ok((source, location))
    }

    fn resolve_path(&self, location: Option<DocumentPath>) -> anyhow::Result<DocumentPath> {
        location
            .or_else(|| {
                self.document
                    .selected_path()
                    .map(|(document_index, path)| DocumentPath {
                        document_index,
                        path,
                    })
            })
            .ok_or_else(|| anyhow::anyhow!("no node is selected"))
    }
}

/// The most recently received result, independent of the command being edited.
struct CommandFeedback {
    name: &'static str,
    message: String,
    kind: FeedbackKind,
}

/// Reusable command editing, execution, and feedback. The host supplies a target and handles
/// effects; this component has no knowledge of a particular view or the viewer's original input.
pub(in crate::viewer) struct CommandSession {
    command: CommandComponent,
    result_feedback: Option<CommandFeedback>,
    config_target: Option<(String, String)>,
}

impl CommandSession {
    pub(in crate::viewer) fn new(
        completer: PathCompleter,
        history: History,
        theme: &ThemeConfig,
    ) -> Self {
        Self {
            command: CommandComponent::new(completer, history, theme.editor.active_char_style),
            result_feedback: None,
            config_target: None,
        }
    }

    pub(in crate::viewer) fn is_active(&self) -> bool {
        self.command.is_active()
    }

    pub(in crate::viewer) fn clear_feedback(&mut self) {
        self.result_feedback = None;
        self.config_target = None;
    }

    pub(in crate::viewer) fn render(&self, width: u16) -> (CreatedGraphemes, CreatedGraphemes) {
        if self.is_active() {
            (
                self.command.create_suggestion_graphemes(width),
                self.command.create_graphemes(),
            )
        } else {
            Default::default()
        }
    }

    pub(in crate::viewer) fn replace_completer(&mut self, completer: PathCompleter) {
        self.command.replace_completer(completer);
    }

    /// Returns Ignored for input owned by the host (document movement, search, etc.).
    pub(in crate::viewer) fn update(
        &mut self,
        event: &ViewEvent<'_>,
        mut target: CommandTarget<'_>,
    ) -> ViewUpdate {
        match event {
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Command(CommandAction::Open)),
                ..
            } => {
                self.clear_feedback();
                self.apply_command_action(CommandAction::Open, &mut target)
            }
            ViewEvent::Action {
                action: Action::Component(ComponentAction::Command(action)),
                ..
            } if self.is_active() => self.apply_command_action(*action, &mut target),
            ViewEvent::Action {
                action: Action::Component(ComponentAction::TextEditor(action)),
                ..
            } if self.is_active() => {
                let outcome = self.command.apply_text_editor_action(*action);
                self.apply_command_outcome(outcome, &mut target)
            }
            ViewEvent::Raw(event) if self.is_active() => {
                let outcome = self.command.handle_input(event);
                self.apply_command_outcome(outcome, &mut target)
            }
            ViewEvent::Tick if self.command.tick() => ViewUpdate::Render,
            _ => ViewUpdate::Ignored,
        }
    }

    /// Applies execution results without requesting further viewer effects.
    pub(in crate::viewer) fn notify(
        &mut self,
        notification: &ViewNotification,
        target: CommandTarget<'_>,
    ) {
        match notification {
            ViewNotification::OpenSucceeded => {
                self.finish_command();
            }
            ViewNotification::OpenFailed(error) => {
                self.command.set_error(error.clone());
            }
            ViewNotification::Navigate {
                document_index,
                path,
            } => {
                self.finish_command();
                if target.document.move_to_path(*document_index, path) {
                    self.command.clear_error();
                } else {
                    self.command
                        .set_error(format!("path not found: @{document_index} {path}"));
                }
            }
            ViewNotification::RecallCommand(command) => {
                self.clear_feedback();
                self.command.recall(command);
            }
            ViewNotification::ConfigValue(message) => {
                self.result_feedback = Some(CommandFeedback {
                    name: "config",
                    message: message.clone(),
                    kind: FeedbackKind::Normal,
                });
                self.finish_command();
            }
            ViewNotification::ConfigTarget(value) => {
                self.config_target = value.clone();
            }
            ViewNotification::ConfigUpdated(update) => {
                self.command
                    .set_active_char_style(update.config.theme.editor.active_char_style);
                self.result_feedback = Some(CommandFeedback {
                    name: "config",
                    message: update.message.clone(),
                    kind: FeedbackKind::Success,
                });
                self.finish_command();
            }
            ViewNotification::ConfigFailed(error) => {
                self.command.clear_error();
                self.result_feedback = Some(CommandFeedback {
                    name: "config",
                    message: error.clone(),
                    kind: FeedbackKind::Error,
                });
            }
            ViewNotification::CopySucceeded(target) => {
                self.result_feedback = Some(CommandFeedback {
                    name: "copy",
                    message: format!("copied {target}"),
                    kind: FeedbackKind::Success,
                });
                self.finish_command();
            }
            ViewNotification::CopyFailed(error) => {
                self.command.clear_error();
                self.result_feedback = Some(CommandFeedback {
                    name: "copy",
                    message: error.clone(),
                    kind: FeedbackKind::Error,
                });
            }
            ViewNotification::WriteSucceeded(destination) => {
                self.result_feedback = Some(CommandFeedback {
                    name: "write",
                    message: format!("saved {destination}"),
                    kind: FeedbackKind::Success,
                });
                self.command.clear_error();
                self.finish_command();
            }
            ViewNotification::WriteFailed(error) => {
                self.command.set_error(error.clone());
            }
        }
    }

    fn apply_command_action(
        &mut self,
        action: CommandAction,
        target: &mut CommandTarget<'_>,
    ) -> ViewUpdate {
        let outcome = self.command.apply_action(action);
        self.apply_command_outcome(outcome, target)
    }

    fn apply_command_outcome(
        &mut self,
        outcome: CommandOutcome,
        target: &mut CommandTarget<'_>,
    ) -> ViewUpdate {
        match outcome {
            CommandOutcome::Editing => self.refresh_config_target(),
            CommandOutcome::Cancelled => {
                self.config_target = None;
                ViewUpdate::Render
            }
            CommandOutcome::Submitted(input) => self.open_command(&input, target),
        }
    }

    fn open_command(&mut self, input: &str, target: &mut CommandTarget<'_>) -> ViewUpdate {
        match Command::parse(input).and_then(|command| self.execute_command(command, target)) {
            Ok(effect) => {
                self.command.clear_error();
                effect
            }
            Err(error) => {
                self.command.set_error(error.to_string());
                ViewUpdate::Render
            }
        }
    }

    fn execute_command(
        &mut self,
        command: Command,
        target: &mut CommandTarget<'_>,
    ) -> anyhow::Result<ViewUpdate> {
        Ok(match command {
            Command::Quit(_) => ViewUpdate::Effect(ViewEffect::Exit),
            Command::Config(config) => ViewUpdate::Effect(ViewEffect::Config(config)),
            Command::Copy(copy) => {
                let content = match copy.target {
                    CopyTarget::Path => target.resolve_path(None).map(|location| location.path),
                    CopyTarget::Value => target.document.selected_value(target.source),
                    CopyTarget::Subtree => target.document.selected_subtree(target.source),
                }?;
                ViewUpdate::Effect(ViewEffect::Copy {
                    target: copy.target,
                    content,
                })
            }
            Command::Flatten(_) => ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Flatten {
                data: target.flatten.clone(),
            })),
            Command::Preview(preview) => {
                let DocumentPath {
                    document_index,
                    path,
                } = target.resolve_path(preview.target)?;
                let content = target
                    .source
                    .text_at(document_index, &path)
                    .map_err(|error| {
                        anyhow::anyhow!("cannot preview @{document_index} {path}: {error}")
                    })?;
                ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Preview {
                    content,
                    document_index,
                    path,
                }))
            }
            Command::Focus(focus) => {
                let (
                    source,
                    DocumentPath {
                        document_index,
                        path,
                    },
                ) = target.subtree(focus.target, "focus")?;
                ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Focus {
                    comparison: Box::new(crate::viewer::ComparisonSource {
                        source: target.source.clone(),
                        origin: None,
                    }),
                    source,
                    document_index,
                    path,
                    display_config: Box::new(target.display_config.clone()),
                }))
            }
            Command::Print(print) => {
                let (source, _) = target.subtree(print.target, "print")?;
                ViewUpdate::Effect(ViewEffect::Print(source))
            }
            Command::Write(write) => {
                let (source, _) = target.subtree(write.target, "write")?;
                ViewUpdate::Effect(ViewEffect::Write {
                    source,
                    destination: write.destination,
                })
            }
            Command::Goto(goto) => {
                if !target
                    .document
                    .move_to_path(goto.target.document_index, &goto.target.path)
                {
                    anyhow::bail!(
                        "path not found: @{} {}",
                        goto.target.document_index,
                        goto.target.path
                    );
                }
                self.finish_command();
                ViewUpdate::Render
            }
            Command::Help(_) => ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Help)),
            Command::Hstr(_) => ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Hstr)),
            Command::Jaq(jaq) => ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq {
                query: jaq.query,
                source: target.source.clone(),
                display_config: Box::new(target.display_config.clone()),
            })),
        })
    }

    pub(in crate::viewer) fn finish_command(&mut self) {
        self.command.finish_submission();
        self.config_target = None;
    }

    fn refresh_config_target(&mut self) -> ViewUpdate {
        match self.command.config_set_target() {
            Some(key)
                if self
                    .config_target
                    .as_ref()
                    .is_some_and(|(current, _)| current == &key) =>
            {
                ViewUpdate::Render
            }
            Some(key) => ViewUpdate::Effect(ViewEffect::InspectConfig(key)),
            None => {
                self.config_target = None;
                ViewUpdate::Render
            }
        }
    }

    pub(in crate::viewer) fn feedback(&self) -> Option<Feedback<'_>> {
        self.command
            .error()
            .map(Feedback::CommandError)
            .or_else(|| self.command.status().map(Feedback::CommandStatus))
            .or_else(|| {
                self.config_target
                    .as_ref()
                    .map(|(key, value)| Feedback::View {
                        name: "config",
                        message: Cow::Owned(format!("editing: {key} = {value}")),
                        hint: Cow::Borrowed(""),
                        kind: FeedbackKind::Normal,
                    })
            })
            .or_else(|| {
                self.result_feedback
                    .as_ref()
                    .map(|feedback| Feedback::View {
                        name: feedback.name,
                        message: Cow::Borrowed(&feedback.message),
                        hint: Cow::Borrowed(""),
                        kind: feedback.kind,
                    })
            })
    }
}

#[cfg(test)]
mod tests {
    use std::{io::Cursor, sync::Arc};

    use promkit::core::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

    use crate::{
        catalog::{FlattenLoadState, Indexer},
        config::{Config, Keybinds},
        input_format::InputFormat,
        viewer::ActionContext,
        viewer::ViewContext,
    };

    use super::*;

    /// A host with its own data and indexes, without constructing a browse view.
    struct Host {
        session: CommandSession,
        document: DocumentComponent,
        source: DocumentSource,
        flatten: FlattenSubscription,
    }

    impl Host {
        fn new(format: InputFormat, content: &str) -> Self {
            let config = Config::default();
            let source = DocumentSource::from_reader(
                format,
                Box::new(Cursor::new(content.as_bytes().to_vec())),
            )
            .unwrap();
            let catalog = Indexer::new(source.clone()).index().unwrap();
            let lines: Vec<_> = catalog
                .documents()
                .iter()
                .flat_map(|doc| doc.entries().iter().map(|entry| doc.flatten_entry(entry)))
                .collect();
            Self {
                session: CommandSession::new(
                    PathCompleter::from_catalog(catalog),
                    History::memory(),
                    &config.theme,
                ),
                document: DocumentComponent::parse(
                    source.content(),
                    DocumentDisplayConfig::from_config(format, &config),
                )
                .unwrap(),
                source,
                flatten: FlattenSubscription::ready_for_test(lines),
            }
        }

        fn update(&mut self, event: ViewEvent<'_>) -> ViewUpdate {
            self.session.update(
                &event,
                CommandTarget {
                    display_config: &self.document.display_config(),
                    document: &mut self.document,
                    source: &self.source,
                    flatten: &self.flatten,
                },
            )
        }

        fn notify(&mut self, notification: ViewNotification) {
            self.session.notify(
                &notification,
                CommandTarget {
                    display_config: &self.document.display_config(),
                    document: &mut self.document,
                    source: &self.source,
                    flatten: &self.flatten,
                },
            );
        }

        fn press(&mut self, key: KeyCode) -> ViewUpdate {
            let event = Event::Key(KeyEvent::new(key, KeyModifiers::NONE));
            match crate::viewer::key_resolution::resolve(
                &Keybinds::default(),
                ViewContext::CommandEditor,
                &event,
            ) {
                Some(action) => self.update(ViewEvent::Action {
                    action,
                    context: ActionContext {
                        click_position: None,
                        is_mouse: false,
                        movement_lines: 1,
                    },
                }),
                None => self.update(ViewEvent::Raw(&event)),
            }
        }

        fn submit(&mut self, command: &str) -> ViewUpdate {
            self.notify(ViewNotification::RecallCommand(command.into()));
            self.press(KeyCode::Enter)
        }
    }

    #[test]
    fn shows_the_latest_result_regardless_of_command_kind() {
        let mut host = Host::new(InputFormat::Json, "{}");
        let theme = Config::default().theme.feedback;
        for (event, expected) in [
            (
                ViewNotification::WriteSucceeded("out.json".into()),
                "[vy:write] saved out.json",
            ),
            (
                ViewNotification::ConfigValue("indent = 2".into()),
                "[vy:config] indent = 2",
            ),
            (
                ViewNotification::CopySucceeded("path".into()),
                "[vy:copy] copied path",
            ),
            (
                ViewNotification::ConfigUpdated(Arc::new(crate::viewer::ConfigUpdate {
                    message: "configuration updated".into(),
                    config: Config::default(),
                })),
                "[vy:config] configuration updated",
            ),
            (
                ViewNotification::CopyFailed("clipboard unavailable".into()),
                "[vy:copy] clipboard unavailable",
            ),
            (
                ViewNotification::WriteSucceeded("other.json".into()),
                "[vy:write] saved other.json",
            ),
            (
                ViewNotification::ConfigFailed("invalid value".into()),
                "[vy:config] invalid value",
            ),
        ] {
            host.notify(event);
            assert_eq!(
                host.session
                    .feedback()
                    .unwrap()
                    .create_graphemes(&theme)
                    .graphemes
                    .to_string(),
                expected,
            );
        }
        assert!(matches!(
            host.session.feedback(),
            Some(Feedback::View {
                kind: FeedbackKind::Error,
                ..
            })
        ));
    }

    #[test]
    fn prioritizes_command_errors_then_editing_target_over_result() {
        let mut host = Host::new(InputFormat::Json, "{}");
        host.notify(ViewNotification::RecallCommand(
            "config set json.indent ".into(),
        ));
        host.notify(ViewNotification::ConfigTarget(Some((
            "json.indent".into(),
            "2".into(),
        ))));
        host.notify(ViewNotification::ConfigFailed("invalid value".into()));
        assert!(host.session.is_active());
        assert!(matches!(host.session.feedback(), Some(Feedback::View {
            name: "config", message, kind: FeedbackKind::Normal, ..
        }) if message == "editing: json.indent = 2"));

        host.notify(ViewNotification::OpenFailed("failed to open".into()));
        assert!(matches!(
            host.session.feedback(),
            Some(Feedback::CommandError("failed to open"))
        ));
        host.session.command.clear_error();
        host.notify(ViewNotification::ConfigTarget(None));
        assert!(matches!(host.session.feedback(), Some(Feedback::View {
            name: "config", message, kind: FeedbackKind::Error, ..
        }) if message == "invalid value"));
    }

    #[test]
    fn opening_or_recalling_a_command_clears_result_and_editing_target() {
        let mut host = Host::new(InputFormat::Json, "{}");
        for recall in [false, true] {
            host.notify(ViewNotification::CopySucceeded("path".into()));
            host.notify(ViewNotification::ConfigTarget(Some((
                "json.indent".into(),
                "2".into(),
            ))));
            if recall {
                host.notify(ViewNotification::RecallCommand("help".into()));
            } else {
                host.update(ViewEvent::Action {
                    action: Action::Component(ComponentAction::Command(CommandAction::Open)),
                    context: ActionContext {
                        click_position: None,
                        is_mouse: false,
                        movement_lines: 1,
                    },
                });
            }
            assert!(host.session.is_active());
            assert!(host.session.feedback().is_none());
            // Clearing the editor must not uncover an older result or editing target.
            host.press(KeyCode::Esc);
            assert!(!host.session.is_active());
            assert!(host.session.feedback().is_none());
        }
    }

    #[test]
    fn quit_exits_and_rejects_arguments_without_exiting() {
        let mut host = Host::new(InputFormat::Json, "{}");
        assert!(matches!(host.submit("quit now"), ViewUpdate::Render));
        assert!(host.session.is_active());
        assert_eq!(
            host.session.command.error(),
            Some("quit does not accept arguments")
        );
        assert!(matches!(
            host.submit("quit"),
            ViewUpdate::Effect(ViewEffect::Exit)
        ));
    }

    #[test]
    fn write_waits_for_save_success_and_keeps_failures_editable() {
        let mut host = Host::new(InputFormat::Json, r#"{} {"value":42}"#);
        let selected = host.document.selected_path();
        let ViewUpdate::Effect(ViewEffect::Write {
            source,
            destination,
        }) = host.submit("write ./out.json @1 .value")
        else {
            panic!("expected a save request");
        };
        assert_eq!(source.content(), b"42");
        assert_eq!(destination, std::path::PathBuf::from("./out.json"));
        assert!(host.session.is_active());
        host.notify(ViewNotification::WriteFailed("permission denied".into()));
        assert_eq!(host.session.command.error(), Some("permission denied"));
        assert!(host.session.is_active());
        assert!(matches!(
            host.press(KeyCode::Enter),
            ViewUpdate::Effect(ViewEffect::Write { .. })
        ));
        host.notify(ViewNotification::WriteSucceeded("./out.json".into()));
        assert!(!host.session.is_active());
        assert_eq!(host.session.command.error(), None);
        assert_eq!(host.document.selected_path(), selected);
        assert!(matches!(
            host.session.feedback(),
            Some(Feedback::View {
                name: "write",
                kind: FeedbackKind::Success,
                ..
            })
        ));

        for command in [
            "write ./out.json @1 .missing",
            "write ./out.json @9 .",
            "write ./out.json @1",
        ] {
            assert!(matches!(host.submit(command), ViewUpdate::Render));
            assert!(host.session.is_active());
            assert!(host.session.command.error().is_some());
            assert_eq!(host.document.selected_path(), selected);
        }
    }

    #[test]
    fn writes_the_selected_node_when_target_is_omitted() {
        for (format, content) in [
            (InputFormat::Json, r#"{} {"value":42}"#),
            (InputFormat::Yaml, "{}\n---\nvalue: 42\n"),
        ] {
            let mut host = Host::new(format, content);
            host.submit("goto @1 .value");
            let selected = host.document.selected_path();
            let ViewUpdate::Effect(ViewEffect::Write {
                source,
                destination,
            }) = host.submit("write ./out.json")
            else {
                panic!("expected a save request");
            };
            assert_eq!(String::from_utf8_lossy(source.content()).trim(), "42");
            assert_eq!(source.format(), format);
            assert_eq!(destination, std::path::PathBuf::from("./out.json"));
            assert_eq!(host.document.selected_path(), selected);
        }
    }

    #[test]
    fn prints_current_and_explicit_paths_without_changing_selection() {
        let mut host = Host::new(
            InputFormat::Json,
            r#"{} {"items":[{"z":9007199254740993,"a":1.2300}],"name":"vy","empty":[],"nil":null}"#,
        );
        host.submit("goto @1 .items[0]");
        let selected = host.document.selected_path();
        for (command, expected) in [
            ("print", r#"{"z":9007199254740993,"a":1.2300}"#),
            ("print @1 .name", r#""vy""#),
            ("print @1 .empty", "[]"),
            ("print @1 .nil", "null"),
            ("print @0 .", "{}"),
        ] {
            let ViewUpdate::Effect(ViewEffect::Print(source)) = host.submit(command) else {
                panic!("expected print output for {command}");
            };
            assert_eq!(source.format(), InputFormat::Json);
            assert_eq!(source.content(), expected.as_bytes());
            assert_eq!(host.document.selected_path(), selected);
        }
    }

    #[test]
    fn print_errors_keep_the_editor_open_and_do_not_produce_output() {
        let mut host = Host::new(InputFormat::Json, r#"{"value":42}"#);
        let selected = host.document.selected_path();
        for command in [
            "print @1 .",
            "print @0 .missing",
            "print @0 .value[",
            "print @0",
        ] {
            assert!(matches!(host.submit(command), ViewUpdate::Render));
            assert!(host.session.is_active());
            assert!(host.session.command.error().is_some());
            assert_eq!(host.document.selected_path(), selected);
        }
        let mut empty = Host::new(InputFormat::Json, "");
        assert!(matches!(empty.submit("print"), ViewUpdate::Render));
        assert_eq!(empty.session.command.error(), Some("no node is selected"));
    }

    #[test]
    fn executes_against_independent_json_and_yaml_targets() {
        for (format, content, path, value) in [
            (
                InputFormat::Json,
                "{\"name\":\"first\"}\n{\"name\":\"second\"}",
                "@1 .name",
                "second",
            ),
            (
                InputFormat::Yaml,
                "name: first\n---\nname: second\n",
                "@1 .name",
                "second",
            ),
        ] {
            let mut host = Host::new(format, content);
            assert!(matches!(
                host.submit(&format!("goto {path}")),
                ViewUpdate::Render
            ));
            assert!(!host.session.is_active());
            assert_eq!(host.document.selected_path(), Some((1, ".name".into())));
            assert!(
                matches!(host.submit("copy path"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == ".name")
            );
            assert!(
                matches!(host.submit("copy value"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content.contains(value))
            );
            assert!(
                matches!(host.submit("copy subtree"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content.contains(value))
            );

            let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq {
                source,
                query,
                display_config,
            })) = host.submit("jaq .name")
            else {
                panic!("jaq must carry the caller's source");
            };
            assert_eq!(source.content(), content.as_bytes());
            assert_eq!(source.format(), format);
            assert_eq!(display_config.format(), format);
            assert_eq!(query, ".name");

            let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Flatten { data })) =
                host.submit("flatten")
            else {
                panic!("flatten must carry the caller's index");
            };
            let (FlattenLoadState::Ready(expected), FlattenLoadState::Ready(actual)) =
                (host.flatten.state(), data.state())
            else {
                panic!("fixture index must be ready");
            };
            assert!(Arc::ptr_eq(&expected, &actual));
        }
    }

    #[test]
    fn completion_and_navigation_use_the_hosts_catalog() {
        let mut first = Host::new(InputFormat::Json, r#"{"first_only":1}"#);
        let mut second = Host::new(InputFormat::Json, r#"{"second_only":2}"#);
        for (host, name) in [(&mut first, "first_only"), (&mut second, "second_only")] {
            host.notify(ViewNotification::RecallCommand("goto @0 .".into()));
            host.press(KeyCode::Tab);
            assert_eq!(
                host.session.render(80).1.graphemes.to_string(),
                format!(":goto @0 .{name} ")
            );
            host.press(KeyCode::Enter);
            assert_eq!(host.document.selected_path(), Some((0, format!(".{name}"))));
        }
        second.submit("goto @0 .first_only");
        assert!(second.session.is_active());
        assert_eq!(
            second.document.selected_path(),
            Some((0, ".second_only".into()))
        );
        assert_eq!(
            second.session.command.error(),
            Some("path not found: @0 .first_only")
        );
    }

    #[test]
    fn keeps_failed_commands_editable_and_waits_for_success_before_closing() {
        let mut host = Host::new(InputFormat::Json, r#"{"value":1}"#);
        assert!(matches!(
            host.submit("jaq .value"),
            ViewUpdate::Effect(ViewEffect::Open(_))
        ));
        assert!(host.session.is_active());
        host.notify(ViewNotification::OpenFailed("failed to open".into()));
        assert_eq!(
            host.session.render(80).1.graphemes.to_string(),
            ":jaq .value "
        );
        assert_eq!(host.session.command.error(), Some("failed to open"));
        assert!(matches!(
            host.press(KeyCode::Enter),
            ViewUpdate::Effect(ViewEffect::Open(_))
        ));
        assert_eq!(host.session.command.error(), None);
        host.notify(ViewNotification::OpenSucceeded);
        assert!(!host.session.is_active());
        assert!(host.session.render(80).1.graphemes.is_empty());

        host.notify(ViewNotification::RecallCommand("copy path".into()));
        host.press(KeyCode::Esc);
        assert!(!host.session.is_active());
        assert!(matches!(
            host.update(ViewEvent::Raw(&Event::Paste("ignored".into()))),
            ViewUpdate::Ignored
        ));
    }
}
