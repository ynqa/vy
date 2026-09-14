use std::{io::Cursor, time::Duration};

use tokio::time::{sleep, timeout};

use super::*;
use promkit::core::{CreatedGraphemes, crossterm::event::Event as TerminalEvent};

use crate::{
    config::Config,
    input_format::InputFormat,
    viewer::{ViewEffect, ViewNotification},
};

fn press(viewer: &mut BrowseView, code: promkit::core::crossterm::event::KeyCode) -> ViewUpdate {
    use promkit::core::crossterm::event::{KeyEvent, KeyModifiers};
    let event = TerminalEvent::Key(KeyEvent::new(code, KeyModifiers::NONE));
    match crate::viewer::key_resolution::resolve(
        &Config::default().keybinds,
        viewer.context(),
        &event,
    ) {
        Some(action) => viewer.update(action_input(action)),
        None => viewer.update(ViewEvent::Raw(&event)),
    }
}

fn create_feedback_graphemes(viewer: &BrowseView) -> CreatedGraphemes {
    viewer
        .feedback()
        .unwrap()
        .create_graphemes(&viewer.feedback_theme)
}

fn hints() -> QueryHints {
    QueryHints::jaq(&Config::default().keybinds)
}

fn action_input(action: Action) -> ViewEvent<'static> {
    ViewEvent::Action {
        action,
        context: crate::viewer::ActionContext {
            click_position: None,
            is_mouse: false,
            movement_lines: 1,
        },
    }
}

#[tokio::test]
async fn displays_the_full_query_error_in_its_status() {
    let source =
        DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(b"null"))).unwrap();
    let mut viewer = BrowseView::jaq(
        ".【".into(),
        &source,
        DocumentDisplayConfig::from_config(InputFormat::Json, &Config::default()),
        hints(),
        History::memory(),
        &Config::default().theme,
    )
    .unwrap();
    timeout(Duration::from_secs(2), async {
        while viewer.is_running() {
            viewer.tick();
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("jaq view error timeout");

    let JaqStatus::Failed(message) = &viewer.jaq_session().status else {
        panic!("jaq status must contain the failure");
    };
    assert!(!message.is_empty());
    let status = create_feedback_graphemes(&viewer);
    assert_eq!(
        status.graphemes.to_string(),
        format!(
            "[vy:jaq] failed: {} — query: .【 — focus: result — s: toggle input — :, Shift+:: edit query / command — Tab: switch focus — Esc: return",
            message.replace("\r\n", "\n").replace('\r', "\n")
        )
    );
    assert_eq!(status.layout.max_height, None);
    assert_eq!(status.layout.width_mode, promkit::core::WidthMode::Wrap);
}

#[tokio::test]
async fn displays_streamed_results_and_completion() {
    let source =
        DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(b"null"))).unwrap();
    let mut json_config = Config::default().json;
    json_config.show_line_numbers = false;
    let mut viewer = BrowseView::jaq(
        "range(0; 3)".into(),
        &source,
        DocumentDisplayConfig::Json(json_config),
        hints(),
        History::memory(),
        &Config::default().theme,
    )
    .unwrap();

    timeout(Duration::from_secs(2), async {
        while viewer.is_running() {
            viewer.tick();
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("jaq view completion timeout");

    assert!(!viewer.is_running());
    assert_eq!(
        viewer
            .documents
            .create_right_graphemes(80, 20)
            .graphemes
            .to_string(),
        "0,\n1,\n2"
    );
    assert!(
        create_feedback_graphemes(&viewer)
            .graphemes
            .to_string()
            .contains("[vy:jaq] completed — query: range(0; 3)")
    );

    viewer.update(action_input(Action::Component(ComponentAction::Document(
        crate::viewer::DocumentAction::Down,
    ))));
    assert_eq!(
        viewer
            .documents
            .create_right_graphemes(80, 20)
            .cursor
            .expect("jaq result has a cursor")
            .row,
        1
    );
}

#[tokio::test]
async fn toggles_a_side_by_side_comparison_with_the_input() {
    let source = DocumentSource::from_reader(
        InputFormat::Json,
        Box::new(Cursor::new(br#"{"items":[1,2]}"#)),
    )
    .unwrap();
    let mut json_config = Config::default().json;
    json_config.show_line_numbers = false;
    let mut viewer = BrowseView::jaq(
        ".items".into(),
        &source,
        DocumentDisplayConfig::Json(json_config),
        hints(),
        History::memory(),
        &Config::default().theme,
    )
    .unwrap();

    timeout(Duration::from_secs(2), async {
        while viewer.is_running() {
            viewer.tick();
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("jaq view completion timeout");

    assert!(!viewer.documents.is_side_by_side());
    press(&mut viewer, promkit::core::crossterm::event::KeyCode::Tab);
    assert_eq!(viewer.context(), ViewContext::Jaq);
    assert_eq!(viewer.documents.focus(), DocumentFocus::Right);
    viewer.update(action_input(Action::View(ViewAction::Jaq(
        JaqViewAction::ToggleSideBySide,
    ))));
    assert!(viewer.documents.is_side_by_side());

    let frame = viewer.render(41, 20);
    let rendered = frame.content.graphemes.to_string();
    assert!(rendered.contains("\"items\""));
    assert!(rendered.contains(" │ ["));

    press(&mut viewer, promkit::core::crossterm::event::KeyCode::Tab);
    assert_eq!(viewer.documents.focus(), DocumentFocus::Left);
    assert!(
        create_feedback_graphemes(&viewer)
            .graphemes
            .to_string()
            .contains("focus: input")
    );

    use promkit::core::crossterm::event::KeyCode;
    press(&mut viewer, KeyCode::Char(':'));
    assert_eq!(viewer.context(), ViewContext::CommandEditor);
    press(&mut viewer, KeyCode::Tab);
    assert_eq!(viewer.context(), ViewContext::CommandEditor);
    press(&mut viewer, KeyCode::Esc);
    assert_eq!(viewer.context(), ViewContext::Jaq);
    assert_eq!(viewer.documents.focus(), DocumentFocus::Left);
    press(&mut viewer, KeyCode::Tab);
    assert_eq!(viewer.documents.focus(), DocumentFocus::Right);
    assert_eq!(viewer.context(), ViewContext::Jaq);
    press(&mut viewer, KeyCode::Tab);
    assert_eq!(viewer.documents.focus(), DocumentFocus::Left);

    viewer.update(action_input(Action::View(ViewAction::Jaq(
        JaqViewAction::ToggleSideBySide,
    ))));
    assert!(!viewer.documents.is_side_by_side());
    assert_eq!(viewer.documents.focus(), DocumentFocus::Right);
    assert!(
        !viewer
            .render(41, 20)
            .content
            .graphemes
            .to_string()
            .contains(" │ ")
    );
}

#[tokio::test]
async fn colon_edits_multiline_queries_and_records_each_execution() {
    use super::focus_tests::submit;
    use promkit::core::crossterm::event::KeyCode;
    let source =
        DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(b"null"))).unwrap();
    let history = History::memory();
    let mut viewer = BrowseView::jaq(
        ".".into(),
        &source,
        DocumentDisplayConfig::from_config(InputFormat::Json, &Config::default()),
        hints(),
        history.clone(),
        &Config::default().theme,
    )
    .unwrap();
    finish(&mut viewer).await;
    assert!(viewer.render(80, 20).editor.graphemes.is_empty());
    press(&mut viewer, KeyCode::Char(':'));
    assert_eq!(viewer.context(), ViewContext::CommandEditor);
    assert_eq!(
        viewer.render(80, 20).editor.graphemes.to_string(),
        ":jaq . "
    );
    viewer.update(ViewEvent::Raw(&TerminalEvent::Paste(" |".into())));
    viewer.update(action_input(Action::Component(
        ComponentAction::TextEditor(crate::viewer::TextEditorAction::InsertNewline),
    )));
    viewer.update(ViewEvent::Raw(&TerminalEvent::Paste("42".into())));
    assert_eq!(
        viewer.render(80, 20).editor.graphemes.logical_lines().len(),
        2
    );
    assert!(matches!(
        press(&mut viewer, KeyCode::Enter),
        ViewUpdate::Render
    ));
    finish(&mut viewer).await;
    assert_eq!(viewer.context(), ViewContext::Jaq);
    assert!(viewer.render(80, 20).editor.graphemes.is_empty());
    assert_eq!(
        String::from_utf8_lossy(viewer.document_source.content()).trim(),
        "42"
    );
    press(&mut viewer, KeyCode::Char(':'));
    assert_eq!(
        viewer.render(80, 20).editor.graphemes.to_string(),
        ":jaq . |\n 42 "
    );
    press(&mut viewer, KeyCode::Esc);
    assert_eq!(history.newest(), vec!["jaq . |\n42"]);
    assert!(matches!(
        submit(&mut viewer, "jaq . | 7"),
        ViewUpdate::Render
    ));
    finish(&mut viewer).await;
    assert_eq!(
        String::from_utf8_lossy(viewer.document_source.content()).trim(),
        "7"
    );
    assert_eq!(history.newest(), vec!["jaq . | 7", "jaq . |\n42"]);
}

#[tokio::test]
async fn displays_yaml_input_results_as_yaml() {
    let source = DocumentSource::from_reader(
        InputFormat::Yaml,
        Box::new(Cursor::new(b"items:\n  - name: first\n  - name: second\n")),
    )
    .unwrap();
    let mut yaml_config = Config::default().yaml;
    yaml_config.show_line_numbers = false;
    let mut viewer = BrowseView::jaq(
        ".items[]".into(),
        &source,
        DocumentDisplayConfig::Yaml(yaml_config),
        hints(),
        History::memory(),
        &Config::default().theme,
    )
    .unwrap();

    timeout(Duration::from_secs(2), async {
        while viewer.is_running() {
            viewer.tick();
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .expect("jaq view completion timeout");

    assert_eq!(
        viewer
            .documents
            .create_right_graphemes(80, 20)
            .graphemes
            .to_string(),
        "name: first\n---\nname: second"
    );

    viewer.update(action_input(Action::View(ViewAction::Jaq(
        JaqViewAction::ToggleSideBySide,
    ))));
    let comparison = viewer.render(41, 20).content.graphemes.to_string();
    assert!(comparison.contains("items:"));
    assert!(comparison.contains(" │ name: first"));
}

#[tokio::test]
async fn cancels_its_executor_when_it_receives_the_cancel_action() {
    let source =
        DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(b"null"))).unwrap();
    let mut viewer = BrowseView::jaq(
        "0 | recurse(. + 1)".into(),
        &source,
        DocumentDisplayConfig::from_config(InputFormat::Json, &Config::default()),
        hints(),
        History::memory(),
        &Config::default().theme,
    )
    .unwrap();

    viewer.update(action_input(Action::View(ViewAction::Jaq(
        JaqViewAction::CancelExecution,
    ))));
    assert!(
        create_feedback_graphemes(&viewer)
            .graphemes
            .to_string()
            .contains("[vy:jaq] ⠋ cancelling")
    );
}

async fn completed(format: InputFormat, input: &str, query: &str) -> BrowseView {
    let config = Config::default();
    let source =
        DocumentSource::from_reader(format, Box::new(Cursor::new(input.as_bytes().to_vec())))
            .unwrap();
    let mut view = BrowseView::jaq(
        query.into(),
        &source,
        DocumentDisplayConfig::from_config(format, &config),
        hints(),
        History::memory(),
        &config.theme,
    )
    .unwrap();
    finish(&mut view).await;
    view
}

async fn finish(view: &mut BrowseView) {
    timeout(Duration::from_secs(2), async {
        while view.needs_tick() {
            view.update(ViewEvent::Tick);
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    super::focus_tests::ready(view).await;
}

#[tokio::test]
async fn jaq_results_support_all_commands_and_local_search_and_completion() {
    use super::focus_tests::{dispatch, ready, submit};
    use promkit::core::crossterm::event::KeyCode;
    for (format, input) in [
        (
            InputFormat::Json,
            r#"{"outside":0,"nested":{"value":42,"next":43}}"#,
        ),
        (
            InputFormat::Yaml,
            "outside: 0\nnested:\n  value: 42\n  next: 43\n",
        ),
    ] {
        let mut view = completed(format, input, ".nested").await;
        press(&mut view, KeyCode::Char(':'));
        view.update(action_input(Action::Component(
            ComponentAction::TextEditor(crate::viewer::TextEditorAction::EraseAll),
        )));
        assert_eq!(view.context(), ViewContext::CommandEditor);
        dispatch(&mut view, TerminalEvent::Paste("goto @0 .val".into()));
        press(&mut view, KeyCode::Tab);
        assert!(
            view.render(100, 20)
                .editor
                .graphemes
                .to_string()
                .contains("goto @0 .value")
        );
        press(&mut view, KeyCode::Enter);
        assert_eq!(view.document.selected_path(), Some((0, ".value".into())));
        for target in ["path", "value", "subtree"] {
            let ViewUpdate::Effect(ViewEffect::Copy { content, .. }) =
                submit(&mut view, &format!("copy {target}"))
            else {
                panic!("copy");
            };
            assert_eq!(
                content.trim(),
                if target == "path" { ".value" } else { "42" }
            );
            view.notify(ViewNotification::CopySucceeded(target.into()));
        }
        let ViewUpdate::Effect(ViewEffect::Write {
            source,
            destination,
        }) = submit(&mut view, "write ./result.json")
        else {
            panic!("write");
        };
        assert_eq!(source.format(), format);
        assert_eq!(String::from_utf8_lossy(source.content()).trim(), "42");
        assert_eq!(destination, std::path::PathBuf::from("./result.json"));
        view.notify(ViewNotification::WriteSucceeded("./result.json".into()));
        let ViewUpdate::Effect(ViewEffect::Print(source)) = submit(&mut view, "print") else {
            panic!("print");
        };
        assert_eq!(String::from_utf8_lossy(source.content()).trim(), "42");
        view.notify(ViewNotification::OpenSucceeded);
        let ViewUpdate::Effect(ViewEffect::Open(request)) = submit(&mut view, "focus") else {
            panic!("focus");
        };
        let focused = super::focus_tests::child(request).await;
        assert_eq!(
            String::from_utf8_lossy(focused.document_source.content()).trim(),
            "42"
        );
        view.notify(ViewNotification::OpenSucceeded);
        assert!(matches!(
            submit(&mut view, "jaq .nested"),
            ViewUpdate::Render
        ));
        finish(&mut view).await;
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Flatten { data })) =
            submit(&mut view, "flatten")
        else {
            panic!("flatten");
        };
        let FlattenLoadState::Ready(data) = data.state() else {
            panic!("index");
        };
        assert!(data.lines().iter().any(|line| line.contains("value")));
        assert!(!data.lines().iter().any(|line| line.contains("outside")));
        view.notify(ViewNotification::OpenSucceeded);
        view.notify(ViewNotification::Navigate {
            document_index: 0,
            path: ".next".into(),
        });
        assert_eq!(view.document.selected_path(), Some((0, ".next".into())));
        for name in ["help", "hstr"] {
            assert!(matches!(
                submit(&mut view, name),
                ViewUpdate::Effect(ViewEffect::Open(_))
            ));
            view.notify(ViewNotification::OpenSucceeded);
        }
        assert!(matches!(
            submit(&mut view, "config view"),
            ViewUpdate::Effect(ViewEffect::Config(_))
        ));
        view.notify(ViewNotification::OpenSucceeded);
        ready(&mut view).await;
        press(&mut view, KeyCode::Char('/'));
        dispatch(&mut view, TerminalEvent::Paste("value".into()));
        press(&mut view, KeyCode::Enter);
        assert_eq!(view.document.selected_path(), Some((0, ".value".into())));
        view.notify(ViewNotification::RecallCommand(
            "write ./history.json".into(),
        ));
        assert!(
            view.render(100, 20)
                .editor
                .graphemes
                .to_string()
                .contains("write ./history.json")
        );
    }
}

#[tokio::test]
async fn commands_follow_the_active_pane_and_remain_pinned_during_resize() {
    use super::focus_tests::{dispatch, ready, submit};
    use promkit::core::crossterm::event::KeyCode;
    let mut view = completed(
        InputFormat::Json,
        r#"{"input_only":1,"nested":{"result_only":2}}"#,
        ".nested",
    )
    .await;
    press(&mut view, KeyCode::Char('s'));
    view.render(100, 20);
    press(&mut view, KeyCode::Tab);
    ready(&mut view).await;
    submit(&mut view, "goto @0 .input_only");
    press(&mut view, KeyCode::Char(':'));
    view.update(action_input(Action::Component(
        ComponentAction::TextEditor(crate::viewer::TextEditorAction::EraseAll),
    )));
    dispatch(&mut view, TerminalEvent::Paste("write ./left.json".into()));
    view.render(4, 20);
    assert_eq!(view.documents.focus(), DocumentFocus::Left);
    let ViewUpdate::Effect(ViewEffect::Write { source, .. }) = press(&mut view, KeyCode::Enter)
    else {
        panic!("write left");
    };
    assert_eq!(source.content(), b"1");
    view.notify(ViewNotification::WriteSucceeded("left.json".into()));
    view.render(100, 20);
    press(&mut view, KeyCode::Tab);
    submit(&mut view, "goto @0 .result_only");
    let ViewUpdate::Effect(ViewEffect::Write { source, .. }) =
        submit(&mut view, "write ./right.json")
    else {
        panic!("write right");
    };
    assert_eq!(source.content(), b"2");
}

#[tokio::test]
async fn repeated_jaq_commands_update_the_same_view_using_the_original_input() {
    use super::focus_tests::submit;
    use promkit::core::crossterm::event::KeyCode;
    let mut view = completed(InputFormat::Json, r#"{"number":3}"#, ".number + 1").await;
    for (command, expected) in [("jaq .number * 2", "6"), ("j .number + 2", "5")] {
        assert!(matches!(submit(&mut view, command), ViewUpdate::Render));
        finish(&mut view).await;
        assert_eq!(
            String::from_utf8_lossy(view.document_source.content()).trim(),
            expected
        );
    }
    assert!(matches!(
        press(&mut view, KeyCode::Esc),
        ViewUpdate::Effect(ViewEffect::Back)
    ));
}

#[tokio::test]
async fn empty_failed_and_cancelled_results_remain_command_capable() {
    use super::focus_tests::submit;
    for query in [
        "empty",
        "error(\"oops\")",
        ".bad syntax",
        "1, error(\"after output\")",
    ] {
        let mut view = completed(InputFormat::Json, "null", query).await;
        assert!(matches!(
            submit(&mut view, "help"),
            ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Help))
        ));
        view.notify(ViewNotification::OpenSucceeded);
        let effect = submit(&mut view, "write ./out.json");
        if query.starts_with("1,") {
            let ViewUpdate::Effect(ViewEffect::Write { source, .. }) = effect else {
                panic!("partial output can be saved");
            };
            assert_eq!(String::from_utf8_lossy(source.content()).trim(), "1");
        } else {
            assert!(matches!(effect, ViewUpdate::Render));
            assert!(
                view.render(100, 20)
                    .feedback
                    .graphemes
                    .to_string()
                    .contains("no node is selected")
            );
        }
    }
}

#[tokio::test]
async fn streaming_keeps_the_command_snapshot_stable_until_editing_finishes() {
    use super::focus_tests::dispatch;
    use promkit::core::crossterm::event::KeyCode;
    let config = Config::default();
    let source =
        DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(b"null"))).unwrap();
    let mut view = BrowseView::jaq(
        "range(0; 200)".into(),
        &source,
        DocumentDisplayConfig::from_config(InputFormat::Json, &config),
        hints(),
        History::memory(),
        &config.theme,
    )
    .unwrap();
    timeout(Duration::from_secs(2), async {
        while view.document_source.content().is_empty() {
            view.update(ViewEvent::Tick);
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    let snapshot = view.document_source.content().to_vec();
    press(&mut view, KeyCode::Char(':'));
    view.update(action_input(Action::Component(
        ComponentAction::TextEditor(crate::viewer::TextEditorAction::EraseAll),
    )));
    dispatch(
        &mut view,
        TerminalEvent::Paste("write ./snapshot.json @0 .".into()),
    );
    timeout(Duration::from_secs(2), async {
        while view.is_running() {
            view.update(ViewEvent::Tick);
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(view.document_source.content(), snapshot);
    let ViewUpdate::Effect(ViewEffect::Write { source, .. }) = press(&mut view, KeyCode::Enter)
    else {
        panic!("snapshot write");
    };
    assert_eq!(String::from_utf8_lossy(source.content()).trim(), "0");
    view.notify(ViewNotification::WriteSucceeded("snapshot.json".into()));
    finish(&mut view).await;
    assert!(String::from_utf8_lossy(view.document_source.content()).contains("199"));
}

#[tokio::test]
async fn nested_focus_has_comparison_and_returns_to_the_jaq_caller() {
    use super::focus_tests::{child, ready, submit};
    use crate::viewer::stack::ViewStack;
    use promkit::core::crossterm::event::KeyCode;
    let mut view = completed(InputFormat::Json, r#"{"outer":{"value":42}}"#, ".outer").await;
    submit(&mut view, "goto @0 .value");
    let ViewUpdate::Effect(ViewEffect::Open(request)) = submit(&mut view, "focus") else {
        panic!("focus");
    };
    view.notify(ViewNotification::OpenSucceeded);
    let mut focus = child(request).await;
    press(&mut focus, KeyCode::Char('s'));
    let rendered = focus.render(100, 20).content.graphemes.to_string();
    assert!(rendered.contains("value"));
    assert!(rendered.contains("focus"));
    press(&mut focus, KeyCode::Tab);
    ready(&mut focus).await;
    submit(&mut focus, "goto @0 .value");
    let ViewUpdate::Effect(ViewEffect::Write { source, .. }) =
        submit(&mut focus, "write ./left.json")
    else {
        panic!("write comparison input");
    };
    assert_eq!(source.content(), b"42");
    focus.notify(ViewNotification::WriteSucceeded("left.json".into()));
    let mut stack = ViewStack::new(Box::new(view));
    stack.push(Box::new(focus));
    assert!(matches!(
        super::focus_tests::press(stack.active_mut(), KeyCode::Esc),
        ViewUpdate::Effect(ViewEffect::Back)
    ));
    stack.back();
    assert!(
        matches!(submit(stack.active_mut(),"copy path"),ViewUpdate::Effect(ViewEffect::Copy {content,..}) if content==".value")
    );
}

#[tokio::test]
async fn config_updates_both_panes_and_reexecution_and_can_disable_jaq_specific_keys() {
    use super::focus_tests::submit;
    use crate::viewer::ConfigUpdate;
    let mut view = completed(InputFormat::Json, r#"{"outer":{"value":42}}"#, ".outer").await;
    let mut config = Config::default();
    config.json.indent = 7;
    config.keybinds.view.jaq.back.clear();
    config.keybinds.view.jaq.toggle_focus.clear();
    config.keybinds.view.jaq.toggle_side_by_side.clear();
    let keybinds = config.keybinds.clone();
    view.notify(ViewNotification::ConfigUpdated(std::sync::Arc::new(
        ConfigUpdate {
            message: "updated".into(),
            config,
        },
    )));
    for pane in view.documents.panes_mut() {
        let DocumentDisplayConfig::Json(display) = pane.document.display_config() else {
            panic!("json");
        };
        assert_eq!(display.indent, 7);
    }
    assert!(matches!(submit(&mut view, "jaq ."), ViewUpdate::Render));
    finish(&mut view).await;
    let DocumentDisplayConfig::Json(display) = view.document.display_config() else {
        panic!("json display config expected");
    };
    assert_eq!(display.indent, 7);
    for code in [
        promkit::core::crossterm::event::KeyCode::Esc,
        promkit::core::crossterm::event::KeyCode::Tab,
        promkit::core::crossterm::event::KeyCode::Char('s'),
    ] {
        use promkit::core::crossterm::event::{KeyEvent, KeyModifiers};
        assert_eq!(
            crate::viewer::key_resolution::resolve(
                &keybinds,
                view.context(),
                &TerminalEvent::Key(KeyEvent::new(code, KeyModifiers::NONE))
            ),
            None
        );
    }
    assert!(matches!(
        submit(&mut view, "jaq .outer.value"),
        ViewUpdate::Render
    ));
    finish(&mut view).await;
    let DocumentDisplayConfig::Json(display) = view.document.display_config() else {
        panic!("json");
    };
    assert_eq!(display.indent, 7);
}

#[tokio::test]
async fn cancelling_preserves_partial_results_and_allows_further_commands() {
    use super::focus_tests::submit;
    let config = Config::default();
    let source =
        DocumentSource::from_reader(InputFormat::Json, Box::new(Cursor::new(b"null"))).unwrap();
    let mut view = BrowseView::jaq(
        "0 | recurse(. + 1)".into(),
        &source,
        DocumentDisplayConfig::from_config(InputFormat::Json, &config),
        hints(),
        History::memory(),
        &config.theme,
    )
    .unwrap();
    timeout(Duration::from_secs(2), async {
        while view.document_source.content().is_empty() {
            view.update(ViewEvent::Tick);
            sleep(Duration::from_millis(1)).await;
        }
    })
    .await
    .unwrap();
    view.update(action_input(Action::View(ViewAction::Jaq(
        JaqViewAction::CancelExecution,
    ))));
    finish(&mut view).await;
    assert!(matches!(view.jaq_session().status, JaqStatus::Cancelled));
    assert!(matches!(
        submit(&mut view, "write ./partial.json @0 ."),
        ViewUpdate::Effect(ViewEffect::Write { .. })
    ));
    view.notify(ViewNotification::WriteSucceeded("partial.json".into()));
    assert!(matches!(
        submit(&mut view, "help"),
        ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Help))
    ));
}

#[tokio::test]
async fn colon_reexecutes_the_current_query_on_the_original_input_from_either_pane() {
    use promkit::core::crossterm::event::KeyCode;
    for (format, input) in [
        (InputFormat::Json, r#"{"outside":99,"nested":{"value":42}}"#),
        (InputFormat::Yaml, "outside: 99\nnested:\n  value: 42\n"),
    ] {
        for left in [false, true] {
            let mut view = completed(format, input, ".nested").await;
            if left {
                press(&mut view, KeyCode::Char('s'));
                view.render(100, 20);
                press(&mut view, KeyCode::Tab);
            }
            press(&mut view, KeyCode::Char(':'));
            assert_eq!(
                view.render(100, 20).editor.graphemes.to_string(),
                ":jaq .nested "
            );
            view.update(ViewEvent::Raw(&TerminalEvent::Paste(" | .value".into())));
            assert!(matches!(
                press(&mut view, KeyCode::Enter),
                ViewUpdate::Render
            ));
            finish(&mut view).await;
            assert_eq!(view.documents.focus(), DocumentFocus::Right);
            assert_eq!(
                view.document
                    .selected_value(&view.document_source)
                    .unwrap()
                    .trim(),
                "42"
            );
            assert_eq!(
                view.documents
                    .panes_mut()
                    .next()
                    .unwrap()
                    .document_source
                    .content(),
                input.as_bytes()
            );
            press(&mut view, KeyCode::Char(':'));
            assert!(
                view.render(100, 20)
                    .editor
                    .graphemes
                    .to_string()
                    .contains(":jaq .nested | .value")
            );
            view.update(ViewEvent::Raw(&TerminalEvent::Paste(" + 100".into())));
            press(&mut view, KeyCode::Esc);
            press(&mut view, KeyCode::Char(':'));
            assert!(
                !view
                    .render(100, 20)
                    .editor
                    .graphemes
                    .to_string()
                    .contains("100")
            );
            press(&mut view, KeyCode::Esc);
            assert!(matches!(
                press(&mut view, KeyCode::Esc),
                ViewUpdate::Effect(ViewEffect::Back)
            ));
        }
    }
}

#[tokio::test]
async fn query_errors_can_be_corrected_without_opening_another_view() {
    use super::focus_tests::submit;
    use promkit::core::crossterm::event::KeyCode;
    let mut view = completed(InputFormat::Json, r#"{"number":3}"#, ".number").await;
    assert!(matches!(submit(&mut view, "jaq"), ViewUpdate::Render));
    assert_eq!(view.context(), ViewContext::CommandEditor);
    assert_eq!(
        String::from_utf8_lossy(view.document_source.content()).trim(),
        "3"
    );
    press(&mut view, KeyCode::Esc);
    assert!(matches!(submit(&mut view, "jaq .["), ViewUpdate::Render));
    finish(&mut view).await;
    assert!(matches!(view.jaq_session().status, JaqStatus::Failed(_)));
    press(&mut view, KeyCode::Char(':'));
    assert!(
        view.render(80, 20)
            .editor
            .graphemes
            .to_string()
            .contains(":jaq .[")
    );
    press(&mut view, KeyCode::Esc);
    assert!(matches!(
        submit(&mut view, "jaq .number + 1"),
        ViewUpdate::Render
    ));
    finish(&mut view).await;
    assert_eq!(
        String::from_utf8_lossy(view.document_source.content()).trim(),
        "4"
    );
    assert!(matches!(
        submit(&mut view, "focus"),
        ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Focus { .. }))
    ));
}

#[tokio::test]
async fn jaq_input_stays_within_the_focus_that_created_it() {
    use super::focus_tests::{child, root, submit};
    let mut root = root(InputFormat::Json, r#"{"outside":99,"nested":{"value":42}}"#).await;
    let ViewUpdate::Effect(ViewEffect::Open(request)) = submit(&mut root, "focus @0 .nested")
    else {
        panic!("focus");
    };
    let mut focus = child(request).await;
    let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq {
        query,
        source,
        display_config,
    })) = submit(&mut focus, "jaq .value")
    else {
        panic!("jaq from focus");
    };
    let config = Config::default();
    let mut jaq = BrowseView::jaq(
        query,
        &source,
        *display_config,
        hints(),
        History::memory(),
        &config.theme,
    )
    .unwrap();
    finish(&mut jaq).await;
    assert!(matches!(
        submit(&mut jaq, "jaq .value + 1"),
        ViewUpdate::Render
    ));
    finish(&mut jaq).await;
    assert_eq!(
        String::from_utf8_lossy(jaq.document_source.content()).trim(),
        "43"
    );
    assert_eq!(
        jaq.documents
            .panes_mut()
            .next()
            .unwrap()
            .document_source
            .content(),
        br#"{"value":42}"#
    );
}

// Access the session only after verifying that the test constructed a jaq view.
impl BrowseView {
    fn jaq_session(&self) -> &super::JaqSession {
        let super::BrowseMode::Jaq(state) = &self.mode else {
            panic!("expected a jaq view");
        };
        &state.session
    }
}
