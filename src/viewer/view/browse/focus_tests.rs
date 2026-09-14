use std::{io::Cursor, time::Duration};

use promkit::core::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::{
    config::Config,
    input_format::InputFormat,
    viewer::{ActionContext, ViewEffect, ViewNotification, feature::QueryHints, stack::ViewStack},
};
use tokio::time::{sleep, timeout};

use super::*;

pub(super) fn dispatch(view: &mut dyn View, event: Event) -> ViewUpdate {
    match crate::viewer::key_resolution::resolve(&Keybinds::default(), view.context(), &event) {
        Some(action) => view.update(ViewEvent::Action {
            action,
            context: ActionContext {
                click_position: None,
                is_mouse: false,
                movement_lines: 1,
            },
        }),
        None => view.update(ViewEvent::Raw(&event)),
    }
}

pub(super) fn press(view: &mut dyn View, code: KeyCode) -> ViewUpdate {
    let modifiers = if matches!(code, KeyCode::Char(c) if c.is_ascii_uppercase()) {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    dispatch(view, Event::Key(KeyEvent::new(code, modifiers)))
}

pub(super) fn submit(view: &mut dyn View, command: &str) -> ViewUpdate {
    press(view, KeyCode::Char(':'));
    view.update(ViewEvent::Action {
        action: Action::Component(ComponentAction::TextEditor(
            crate::viewer::TextEditorAction::EraseAll,
        )),
        context: ActionContext {
            click_position: None,
            is_mouse: false,
            movement_lines: 1,
        },
    });
    dispatch(view, Event::Paste(command.into()));
    press(view, KeyCode::Enter)
}

pub(super) async fn ready(view: &mut BrowseView) {
    timeout(Duration::from_secs(2), async {
        while matches!(view.navigation.state(), FlattenLoadState::Loading) {
            view.navigation.changed().await;
        }
    })
    .await
    .expect("focus index timeout");
    assert!(matches!(
        view.navigation.state(),
        FlattenLoadState::Ready(_)
    ));
}

pub(super) async fn root(format: InputFormat, content: &str) -> BrowseView {
    let config = Config::default();
    let source =
        DocumentSource::from_reader(format, Box::new(Cursor::new(content.as_bytes().to_vec())))
            .unwrap();
    let catalog = CatalogLoader::spawn(source.clone());
    let flatten = FlattenLoader::spawn(catalog.clone());
    let document = DocumentComponent::parse(
        source.content(),
        DocumentDisplayConfig::from_config(format, &config),
    )
    .unwrap();
    let mut view = BrowseView::new(
        document,
        source,
        PathCompleter::new(catalog),
        flatten,
        History::memory(),
        &config.theme,
    );
    ready(&mut view).await;
    view
}

pub(super) async fn child(request: ViewRequest) -> BrowseView {
    let ViewRequest::Focus {
        comparison,
        source,
        document_index,
        path,
        display_config,
    } = request
    else {
        panic!("expected focus request");
    };
    let config = Config::default();
    let mut view = BrowseView::focused(
        source,
        (document_index, path),
        *display_config,
        History::memory(),
        &config.keybinds,
        &config.theme,
    )
    .unwrap()
    .with_comparison(comparison.source, comparison.origin)
    .unwrap();
    ready(&mut view).await;
    view
}

async fn open(parent: &mut BrowseView) -> BrowseView {
    let ViewUpdate::Effect(ViewEffect::Open(request)) = submit(parent, "focus") else {
        panic!("focus must open a view");
    };
    let view = child(request).await;
    parent.notify(ViewNotification::OpenSucceeded);
    view
}

#[tokio::test]
async fn explicit_focus_paths_preserve_selection_and_resolve_within_each_view() {
    for (format, content) in [
        (
            InputFormat::Json,
            r#"{} {"outside":99,"items":[{"名前":{"value":42}}]}"#,
        ),
        (
            InputFormat::Yaml,
            "{}\n---\noutside: 99\nitems:\n  - 名前:\n      value: 42\n",
        ),
    ] {
        let mut input = root(format, content).await;
        let selected = input.document.selected_path();
        let before = input.render(80, 10).content.graphemes;
        let ViewUpdate::Effect(ViewEffect::Open(request)) =
            submit(&mut input, "focus @1 .items[0]")
        else {
            panic!("explicit path must open a focus view");
        };
        let mut focus = child(request).await;
        input.notify(ViewNotification::OpenSucceeded);
        assert_eq!(input.document.selected_path(), selected);
        assert_eq!(input.render(80, 10).content.graphemes, before);
        assert_eq!(focus.origin, Some((1, ".items[0]".into())));
        assert_eq!(focus.document_source.format(), format);
        let focused_selection = focus.document.selected_path();

        // Both completion and execution use the subtree's local document and keys.
        press(&mut focus, KeyCode::Char(':'));
        dispatch(&mut focus, Event::Paste(r#"focus @0 ["名"#.into()));
        press(&mut focus, KeyCode::Tab);
        assert_eq!(
            focus.render(80, 10).editor.graphemes.to_string(),
            ":focus @0 [\"名前\"] "
        );
        let ViewUpdate::Effect(ViewEffect::Open(request)) = press(&mut focus, KeyCode::Enter)
        else {
            panic!("completed path must open a nested focus view");
        };
        let mut nested = child(request).await;
        focus.notify(ViewNotification::OpenSucceeded);
        assert_eq!(nested.origin, Some((1, r#".items[0]["名前"]"#.into())));
        assert!(matches!(
            submit(&mut nested, "copy subtree"),
            ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content.contains("42") && !content.contains("outside")
        ));

        for command in [
            "focus @0 .outside",
            "focus @1 .",
            "focus @0 .items[",
            "focus @0 .missing",
        ] {
            assert!(matches!(submit(&mut focus, command), ViewUpdate::Render));
            assert_eq!(focus.context(), ViewContext::CommandEditor);
            assert!(
                focus
                    .render(80, 10)
                    .feedback
                    .graphemes
                    .to_string()
                    .contains("cannot focus")
            );
            press(&mut focus, KeyCode::Esc);
            assert_eq!(focus.document.selected_path(), focused_selection);
        }
    }
}

#[tokio::test]
async fn write_resolves_paths_within_focus_and_continues_browsing() {
    for (format, content) in [
        (
            InputFormat::Json,
            r#"{} {"outside":99,"nested":{"value":42}}"#,
        ),
        (
            InputFormat::Yaml,
            "{}\n---\noutside: 99\nnested:\n  value: 42\n",
        ),
    ] {
        let mut root = root(format, content).await;
        let ViewUpdate::Effect(ViewEffect::Open(request)) = submit(&mut root, "focus @1 .nested")
        else {
            panic!("expected focus");
        };
        let mut focus = child(request).await;
        press(&mut focus, KeyCode::Char(':'));
        dispatch(&mut focus, Event::Paste("write ./out.json @0 .val".into()));
        press(&mut focus, KeyCode::Tab);
        let ViewUpdate::Effect(ViewEffect::Write {
            source,
            destination,
        }) = press(&mut focus, KeyCode::Enter)
        else {
            panic!("expected save request from focus");
        };
        assert_eq!(source.format(), format);
        assert_eq!(String::from_utf8_lossy(source.content()).trim(), "42");
        assert_eq!(destination, std::path::PathBuf::from("./out.json"));
        focus.notify(ViewNotification::WriteSucceeded("./out.json".into()));
        assert_eq!(
            focus.context(),
            ViewContext::Focus(crate::viewer::BrowseFocus::Document)
        );
        assert!(matches!(
            submit(&mut focus, "write ./out.json @1 ."),
            ViewUpdate::Render
        ));
        assert_eq!(focus.context(), ViewContext::CommandEditor);
    }
}

#[tokio::test]
async fn print_uses_the_focused_source_for_selection_paths_and_completion() {
    for (format, content) in [
        (
            InputFormat::Json,
            r#"{} {"outside":99,"nested":{"value":42}}"#,
        ),
        (
            InputFormat::Yaml,
            "{}\n---\noutside: 99\nnested:\n  value: 42\n",
        ),
    ] {
        for explicit in [false, true] {
            let mut input = root(format, content).await;
            let ViewUpdate::Effect(ViewEffect::Open(request)) =
                submit(&mut input, "focus @1 .nested")
            else {
                panic!("expected focus");
            };
            let mut focus = child(request).await;
            let effect = if explicit {
                press(&mut focus, KeyCode::Char(':'));
                dispatch(&mut focus, Event::Paste("print @0 .val".into()));
                press(&mut focus, KeyCode::Tab);
                assert_eq!(
                    focus.render(80, 10).editor.graphemes.to_string(),
                    ":print @0 .value "
                );
                press(&mut focus, KeyCode::Enter)
            } else {
                submit(&mut focus, "goto @0 .value");
                submit(&mut focus, "print")
            };
            let ViewUpdate::Effect(ViewEffect::Print(source)) = effect else {
                panic!("expected print output from the focused data");
            };
            assert_eq!(source.format(), format);
            assert_eq!(std::str::from_utf8(source.content()).unwrap().trim(), "42");
        }
    }
}

#[tokio::test]
async fn jaq_executes_on_the_focused_node_in_json_and_yaml() {
    for (format, content) in [
        (
            InputFormat::Json,
            r#"{"name":"OTHER_DOCUMENT"} {"name":"OUTSIDE_ROOT","items":[{"name":"SKIP"},{"name":"FOCUS_ONLY","nested":{"value":42}}]}"#,
        ),
        (
            InputFormat::Yaml,
            "name: OTHER_DOCUMENT\n---\nname: OUTSIDE_ROOT\nitems:\n  - name: SKIP\n  - name: FOCUS_ONLY\n    nested:\n      value: 42\n",
        ),
    ] {
        let mut input = root(format, content).await;
        assert!(input.document.move_to_path(1, ".items[1]"));
        let mut focus = open(&mut input).await;
        assert_eq!(focus.context(), ViewContext::Focus(BrowseFocus::Document));
        assert!(
            focus
                .render(100, 20)
                .feedback
                .graphemes
                .to_string()
                .contains("@1 .items[1]")
        );
        assert_eq!(focus.document_source.format(), format);
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq {
            source,
            query,
            display_config,
        })) = submit(&mut focus, "jaq .name")
        else {
            panic!("jaq request expected");
        };
        let config = Config::default();
        let mut jaq = BrowseView::jaq(
            query,
            &source,
            *display_config,
            QueryHints::jaq(&config.keybinds),
            History::memory(),
            &config.theme,
        )
        .unwrap();
        timeout(Duration::from_secs(2), async {
            while jaq.is_running() {
                jaq.tick();
                sleep(Duration::from_millis(1)).await;
            }
        })
        .await
        .expect("jaq timeout");
        let frame = jaq.render(100, 20);
        assert!(frame.feedback.graphemes.to_string().contains("completed"));
        let output = frame.content.graphemes.to_string();
        assert!(output.contains("FOCUS_ONLY"), "{output}");
        assert!(!output.contains("OUTSIDE_ROOT"));
        assert!(!output.contains("OTHER_DOCUMENT"));
        assert!(!output.contains("SKIP"));
    }
}

#[tokio::test]
async fn completion_search_copy_and_navigation_are_local_to_focus() {
    let mut input = root(
        InputFormat::Json,
        r#"{"outside":99,"items":[{"name":"first","nested":{"value":42}},{"name":"second"}]}"#,
    )
    .await;
    input.document.move_to_path(0, ".items[0]");
    let mut focus = open(&mut input).await;
    // Completion uses the new subtree's catalog, including local document index zero.
    press(&mut focus, KeyCode::Char(':'));
    dispatch(&mut focus, Event::Paste("goto @0 .na".into()));
    press(&mut focus, KeyCode::Tab);
    assert_eq!(
        focus.render(100, 20).editor.graphemes.to_string(),
        ":goto @0 .name "
    );
    press(&mut focus, KeyCode::Enter);
    assert_eq!(focus.document.selected_path(), Some((0, ".name".into())));
    assert!(
        matches!(submit(&mut focus, "copy path"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == ".name")
    );
    focus.notify(ViewNotification::CopySucceeded("path".into()));
    assert!(
        matches!(submit(&mut focus, "copy value"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == "first")
    );
    focus.notify(ViewNotification::CopySucceeded("value".into()));
    submit(&mut focus, "goto @0 .outside");
    assert_eq!(focus.context(), ViewContext::CommandEditor);
    assert!(
        focus
            .render(100, 20)
            .feedback
            .graphemes
            .to_string()
            .contains("path not found")
    );
    press(&mut focus, KeyCode::Esc);
    assert_eq!(focus.context(), ViewContext::Focus(BrowseFocus::Document));
    submit(&mut focus, "goto @1 .name");
    assert_eq!(focus.context(), ViewContext::CommandEditor);
    press(&mut focus, KeyCode::Esc);

    submit(&mut focus, "goto @0 .");
    press(&mut focus, KeyCode::Char('H'));
    press(&mut focus, KeyCode::Char('J'));
    assert_eq!(focus.document.selected_path(), Some((0, ".".into())));
    press(&mut focus, KeyCode::Char('E'));
    press(&mut focus, KeyCode::Char('/'));
    dispatch(&mut focus, Event::Paste("value".into()));
    press(&mut focus, KeyCode::Enter);
    assert_eq!(
        focus.document.selected_path(),
        Some((0, ".nested.value".into()))
    );
    assert!(
        focus
            .render(100, 20)
            .feedback
            .graphemes
            .to_string()
            .contains("1/1")
    );
    let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Flatten { data })) =
        submit(&mut focus, "flatten")
    else {
        panic!("flatten request expected");
    };
    let FlattenLoadState::Ready(data) = data.state() else {
        panic!("index must be ready");
    };
    assert!(
        data.lines()
            .iter()
            .any(|line| line.contains("@0.nested.value"))
    );
    assert!(
        !data.lines().iter().any(|line| line.contains("outside")
            || line.contains("items")
            || line.contains("second"))
    );
}

#[tokio::test]
async fn nested_focus_returns_one_level_at_a_time_and_restores_each_view() {
    let mut input = root(
        InputFormat::Json,
        r#"{} {"before":0,"items":[{}, {"first]name":{"value":42},"tail":3}],"after":4}"#,
    )
    .await;
    input.document.move_to_path(1, ".items[1]");
    input.document.collapse_selected();
    let original = input.render(80, 6).content.graphemes;
    let mut focus = open(&mut input).await;
    submit(&mut focus, r#"goto @0 ["first]name"]"#);
    let focused = focus.render(80, 6).content.graphemes;
    let nested = open(&mut focus).await;
    assert_eq!(
        nested.origin,
        Some((1, r#".items[1]["first]name"]"#.into()))
    );
    let mut stack = ViewStack::new(Box::new(input));
    stack.push(Box::new(focus));
    stack.push(Box::new(nested));
    press(stack.active_mut(), KeyCode::Char('E'));
    assert!(matches!(
        press(stack.active_mut(), KeyCode::Esc),
        ViewUpdate::Effect(ViewEffect::Back)
    ));
    stack.back();
    assert_eq!(stack.active_mut().render(80, 6).content.graphemes, focused);
    assert!(matches!(
        press(stack.active_mut(), KeyCode::Esc),
        ViewUpdate::Effect(ViewEffect::Back)
    ));
    stack.back();
    assert_eq!(stack.active_mut().render(80, 6).content.graphemes, original);
    assert_eq!(
        stack.active().context(),
        ViewContext::Input(BrowseFocus::Document)
    );
}

#[tokio::test]
async fn supports_scalars_empty_containers_and_rejects_invalid_focus() {
    for (format, source) in [
        (
            InputFormat::Json,
            r#"{"value":null,"empty":[],"object":{}}"#,
        ),
        (InputFormat::Yaml, "value: 'true'\nempty: []\nobject: {}\n"),
    ] {
        let mut input = root(format, source).await;
        for path in [".value", ".empty", ".object"] {
            assert!(input.document.move_to_path(0, path));
            let mut focus = open(&mut input).await;
            assert!(!focus.render(80, 10).content.graphemes.is_empty());
            assert!(matches!(
                press(&mut focus, KeyCode::Esc),
                ViewUpdate::Effect(ViewEffect::Back)
            ));
        }
    }
    let mut empty = root(InputFormat::Json, "").await;
    assert!(matches!(submit(&mut empty, "focus"), ViewUpdate::Render));
    assert!(
        empty
            .render(80, 10)
            .feedback
            .graphemes
            .to_string()
            .contains("no node is selected")
    );
    press(&mut empty, KeyCode::Esc);
    submit(&mut empty, "focus .items");
    assert!(
        empty
            .render(80, 10)
            .feedback
            .graphemes
            .to_string()
            .contains("requires a document index and path")
    );
}

#[tokio::test]
async fn flatten_returns_to_focus_and_preserves_the_original_view() {
    use crate::viewer::view::FlattenView;
    let mut input = root(
        InputFormat::Json,
        r#"{"items":[{"name":"local"}],"outside":true}"#,
    )
    .await;
    input.document.move_to_path(0, ".items[0]");
    let mut focus = open(&mut input).await;
    let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Flatten { data })) =
        submit(&mut focus, "flatten")
    else {
        panic!("flatten expected");
    };
    focus.notify(ViewNotification::OpenSucceeded);
    let config = Config::default();
    let flatten = FlattenView::new(data, QueryHints::flatten(&config.keybinds), &config.theme);
    let mut stack = ViewStack::new(Box::new(input));
    stack.push(Box::new(focus));
    stack.push(Box::new(flatten));
    press(stack.active_mut(), KeyCode::Char('j'));
    let ViewUpdate::Effect(ViewEffect::Navigate {
        document_index,
        path,
    }) = press(stack.active_mut(), KeyCode::Enter)
    else {
        panic!("flatten selection must navigate");
    };
    assert_eq!((document_index, path.as_str()), (0, ".name"));
    stack.return_to_caller(ViewNotification::Navigate {
        document_index,
        path,
    });
    assert_eq!(
        stack.active().context(),
        ViewContext::Focus(BrowseFocus::Document)
    );
    assert!(
        matches!(submit(stack.active_mut(), "copy value"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == "local")
    );
    stack
        .active_mut()
        .notify(ViewNotification::CopySucceeded("value".into()));
    assert!(matches!(
        press(stack.active_mut(), KeyCode::Esc),
        ViewUpdate::Effect(ViewEffect::Back)
    ));
    stack.back();
    assert_eq!(
        stack.active().context(),
        ViewContext::Input(BrowseFocus::Document)
    );
    assert!(
        matches!(submit(stack.active_mut(), "copy path"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == ".items[0]")
    );
}

#[tokio::test]
async fn settings_changed_in_focus_also_update_the_suspended_view() {
    use crate::viewer::ConfigUpdate;
    let mut input = root(InputFormat::Json, r#"{"nested":{"name":"local"}}"#).await;
    input.document.move_to_path(0, ".nested");
    let focus = open(&mut input).await;
    let mut stack = ViewStack::new(Box::new(input));
    stack.push(Box::new(focus));
    let mut config = Config::default();
    config.json.indent = 4;
    config.json.show_child_count = false;
    stack.update_config(ConfigUpdate {
        message: "json.indent = 4".into(),
        config,
    });
    for _ in 0..2 {
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Jaq { display_config, .. })) =
            submit(stack.active_mut(), "jaq .")
        else {
            panic!("jaq request expected");
        };
        let DocumentDisplayConfig::Json(config) = *display_config else {
            panic!("json display config expected");
        };
        assert_eq!(config.indent, 4);
        assert!(!config.show_child_count);
        stack.active_mut().notify(ViewNotification::OpenSucceeded);
        stack.back();
    }
    assert_eq!(
        stack.active().context(),
        ViewContext::Input(BrowseFocus::Document)
    );
    assert!(
        matches!(submit(stack.active_mut(), "copy path"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == ".nested")
    );
}
