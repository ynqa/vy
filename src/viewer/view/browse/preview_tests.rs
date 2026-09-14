use super::focus_tests::{child, press, root, submit};
use super::*;
use promkit::core::crossterm::event::KeyCode;

use crate::{
    config::Config,
    input_format::InputFormat,
    viewer::{ViewEffect, ViewNotification, stack::ViewStack, view::PreviewView},
};

#[tokio::test]
async fn decodes_strings_and_returns_to_the_unchanged_parent() {
    for (format, source) in [
        (
            InputFormat::Json,
            r#"{} {"nested":{"message":"first\nsecond\\n\n"}}"#,
        ),
        (
            InputFormat::Yaml,
            "{}\n---\nnested:\n  message: !Text |\n    first\n    second\\n\n",
        ),
    ] {
        let mut input = root(format, source).await;
        assert!(input.document.move_to_path(1, ".nested"));
        input.document.toggle();
        let before = input.render(120, 24).content.graphemes;
        let selection = input.document.selected_path();
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Preview {
            content,
            document_index,
            path,
        })) = submit(&mut input, "preview @1 .nested.message")
        else {
            panic!("preview request expected");
        };
        assert_eq!(content, "first\nsecond\\n\n");
        assert_eq!((document_index, path.as_str()), (1, ".nested.message"));
        input.notify(ViewNotification::OpenSucceeded);
        assert_eq!(input.document.selected_path(), selection);
        let config = Config::default();
        let preview = PreviewView::new(
            content.clone(),
            document_index,
            path,
            &config.keybinds,
            &config.theme,
        );
        let mut stack = ViewStack::new(Box::new(input));
        stack.push(Box::new(preview));
        assert_eq!(
            stack
                .active_mut()
                .render(120, 24)
                .content
                .graphemes
                .to_string(),
            content
        );
        assert!(matches!(
            press(stack.active_mut(), KeyCode::Esc),
            ViewUpdate::Effect(ViewEffect::Back)
        ));
        stack.back();
        assert_eq!(stack.active_mut().render(120, 24).content.graphemes, before);
        assert!(
            matches!(submit(stack.active_mut(), "copy path"), ViewUpdate::Effect(ViewEffect::Copy { content, .. }) if content == ".nested")
        );
    }
}

#[tokio::test]
async fn uses_the_selected_string_and_preserves_nested_focus_origins() {
    for (format, source) in [
        (
            InputFormat::Json,
            r#"{"nested":{"message":"{\"users\":[1,2]}"}}"#,
        ),
        (
            InputFormat::Yaml,
            "nested:\n  message: '{\"users\":[1,2]}'\n",
        ),
    ] {
        let mut input = root(format, source).await;
        input.document.move_to_path(0, ".nested");
        let ViewUpdate::Effect(ViewEffect::Open(request)) = submit(&mut input, "focus") else {
            panic!("focus expected");
        };
        let mut focus = child(request).await;
        focus.document.move_to_path(0, ".message");
        let ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Preview {
            content,
            document_index,
            path,
        })) = submit(&mut focus, "preview")
        else {
            panic!("preview request expected");
        };
        assert_eq!(content, r#"{"users":[1,2]}"#);
        assert_eq!((document_index, path.as_str()), (0, ".nested.message"));
    }
}

#[tokio::test]
async fn rejects_non_strings_and_missing_paths_without_navigating() {
    for format in [InputFormat::Json, InputFormat::Yaml] {
        let mut input = root(
            format,
            r#"{"number":1,"boolean":true,"null":null,"array":[],"object":{},"empty":""}"#,
        )
        .await;
        for path in [".number", ".boolean", r#"["null"]"#, ".array", ".object"] {
            assert!(input.document.move_to_path(0, path), "{format:?}: {path}");
            assert!(matches!(submit(&mut input, "preview"), ViewUpdate::Render));
            assert!(
                input
                    .render(120, 24)
                    .feedback
                    .graphemes
                    .to_string()
                    .contains("requires a string node")
            );
            assert_eq!(input.document.selected_path(), Some((0, path.into())));
            press(&mut input, KeyCode::Esc);
        }
        assert!(matches!(
            submit(&mut input, "preview @0 .missing"),
            ViewUpdate::Render
        ));
        press(&mut input, KeyCode::Esc);
        assert!(
            matches!(submit(&mut input, "preview @0 .empty"), ViewUpdate::Effect(ViewEffect::Open(ViewRequest::Preview { content, .. })) if content.is_empty())
        );
    }
}
