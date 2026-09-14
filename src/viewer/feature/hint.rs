//! Config-aware operation hints shared by vy's standalone views.

use std::collections::HashSet;

use promkit::core::crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};
use termcfg::event::{event_def::EventDef, format::event_to_shortcut};

use crate::config::Keybinds;

const RETURN: &str = "return";
const CANCEL: &str = "cancel";
const EXECUTE: &str = "execute";
const GOTO: &str = "goto";
const RECALL: &str = "recall";
const SWITCH_FOCUS: &str = "switch focus";
const TOGGLE_INPUT: &str = "toggle input";
const GREP_OPERATORS: &str = "|: OR, &: AND";
const TRY_AGAIN_SHORTLY: &str = "try again shortly";
const USE_CONFIGURED_DOCUMENT_MOVEMENT_KEYS: &str = "use configured document movement keys";

#[derive(Clone, Copy)]
pub(in crate::viewer) enum HintFocus {
    Content,
    Editor,
    Input,
    Result,
}

impl HintFocus {
    fn label(self) -> &'static str {
        match self {
            Self::Content => "content",
            Self::Editor => "editor",
            Self::Input => "input",
            Self::Result => "result",
        }
    }
}

#[derive(Clone, Copy)]
pub(in crate::viewer) enum QueryHintState {
    Ready,
    Running,
    Finished,
}

#[derive(Clone)]
pub(in crate::viewer) struct QueryHints {
    back: Option<String>,
    cancel: Option<String>,
    execute: Option<String>,
    editor_focus: Option<String>,
    open_query_editor: Option<String>,
    open_command_editor: Option<String>,
    editor_back: Option<String>,
    toggle_focus: Option<String>,
    toggle_side_by_side: Option<String>,
    goto_selection: Option<String>,
    grep_operators: bool,
}

impl QueryHints {
    pub(in crate::viewer) fn hstr(keybinds: &Keybinds) -> Self {
        Self {
            back: action_hint(&keybinds.view.hstr.back, RETURN),
            cancel: None,
            execute: None,
            editor_focus: action_hint(&keybinds.view.hstr.toggle_focus, SWITCH_FOCUS),
            open_query_editor: None,
            open_command_editor: None,
            editor_back: action_hint(&keybinds.view.hstr.back, RETURN),
            toggle_focus: action_hint(&keybinds.view.hstr.toggle_focus, SWITCH_FOCUS),
            toggle_side_by_side: None,
            goto_selection: action_hint(&keybinds.view.hstr.recall_selection, RECALL),
            grep_operators: false,
        }
    }

    pub(in crate::viewer) fn flatten(keybinds: &Keybinds) -> Self {
        Self {
            back: action_hint(&keybinds.view.flatten.back, RETURN),
            cancel: action_hint(&keybinds.view.flatten.cancel_execution, CANCEL),
            execute: action_hint(&keybinds.view.flatten.execute_query, EXECUTE),
            editor_focus: action_hint(&keybinds.view.flatten.close_query_editor, "close editor"),
            open_query_editor: action_hint(&keybinds.view.flatten.open_query_editor, "edit query"),
            open_command_editor: None,
            editor_back: None,
            toggle_focus: None,
            toggle_side_by_side: None,
            goto_selection: action_hint(&keybinds.view.flatten.goto_selection, GOTO),
            grep_operators: true,
        }
    }

    pub(in crate::viewer) fn jaq(keybinds: &Keybinds) -> Self {
        Self {
            back: action_hint(&keybinds.view.jaq.back, RETURN),
            cancel: action_hint(&keybinds.view.jaq.cancel_execution, CANCEL),
            execute: action_hint(&keybinds.view.browse.submit_command, EXECUTE),
            editor_focus: action_hint(&keybinds.view.browse.close_command_editor, "close editor"),
            open_query_editor: None,
            open_command_editor: action_hint(
                &keybinds.view.browse.open_command_editor,
                "edit query / command",
            ),
            editor_back: None,
            toggle_focus: action_hint(&keybinds.view.jaq.toggle_focus, SWITCH_FOCUS),
            toggle_side_by_side: action_hint(&keybinds.view.jaq.toggle_side_by_side, TOGGLE_INPUT),
            goto_selection: None,
            grep_operators: false,
        }
    }

    pub(in crate::viewer) fn create(&self, focus: HintFocus, state: QueryHintState) -> String {
        let mut hints = vec![format!("focus: {}", focus.label())];
        match state {
            QueryHintState::Ready | QueryHintState::Finished => {
                if matches!(focus, HintFocus::Editor) {
                    push(&mut hints, &self.execute);
                } else if matches!(focus, HintFocus::Result) {
                    push(&mut hints, &self.goto_selection);
                }
                if matches!(focus, HintFocus::Input | HintFocus::Result) {
                    push(&mut hints, &self.toggle_side_by_side);
                }
            }
            QueryHintState::Running => {
                if matches!(focus, HintFocus::Editor) {
                    push(&mut hints, &self.execute);
                }
                push(&mut hints, &self.cancel);
                if matches!(focus, HintFocus::Input | HintFocus::Result) {
                    push(&mut hints, &self.toggle_side_by_side);
                }
            }
        }
        if matches!(focus, HintFocus::Editor) {
            push(&mut hints, &self.editor_focus);
            push(&mut hints, &self.editor_back);
        } else {
            push(&mut hints, &self.open_command_editor);
            push(&mut hints, &self.open_query_editor);
            push(&mut hints, &self.toggle_focus);
            push(&mut hints, &self.back);
        }
        if self.grep_operators
            && matches!(focus, HintFocus::Editor)
            && matches!(state, QueryHintState::Ready)
        {
            hints.push(GREP_OPERATORS.to_owned());
        }
        hints.join(" — ")
    }

    pub(in crate::viewer) fn loading(&self) -> String {
        self.back.clone().unwrap_or_default()
    }

    pub(in crate::viewer) fn preparing(&self) -> String {
        let mut hints = vec![TRY_AGAIN_SHORTLY.to_owned()];
        push(&mut hints, &self.back);
        hints.join(" — ")
    }
}

#[derive(Clone)]
pub(in crate::viewer) struct HelpHints {
    back: Option<String>,
}

impl HelpHints {
    pub(in crate::viewer) fn new(keybinds: &Keybinds) -> Self {
        Self {
            back: action_hint(&keybinds.view.help.back, RETURN),
        }
    }

    pub(in crate::viewer) fn create(&self) -> String {
        let mut hints = vec![
            format!("focus: {}", HintFocus::Content.label()),
            USE_CONFIGURED_DOCUMENT_MOVEMENT_KEYS.to_owned(),
        ];
        push(&mut hints, &self.back);
        hints.join(" — ")
    }
}

fn action_hint(events: &HashSet<Event>, description: &str) -> Option<String> {
    (!events.is_empty()).then(|| format!("{}: {description}", format_bindings(events)))
}

fn push(hints: &mut Vec<String>, hint: &Option<String>) {
    if let Some(hint) = hint {
        hints.push(hint.clone());
    }
}

pub(in crate::viewer) fn format_bindings(events: &HashSet<Event>) -> String {
    if events.is_empty() {
        return "(disabled)".to_owned();
    }

    let mut shortcuts = events
        .iter()
        .map(|event| {
            let shortcut = EventDef::try_from(event)
                .map(event_to_shortcut)
                .unwrap_or_else(|_| "(unsupported)".to_owned());
            match event {
                Event::Key(KeyEvent {
                    code: KeyCode::Char(character),
                    modifiers: KeyModifiers::NONE,
                    ..
                }) if character.is_ascii_alphabetic() => character.to_string(),
                _ => shortcut,
            }
        })
        .collect::<Vec<_>>();
    shortcuts.sort();
    shortcuts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_query_hints_from_configured_bindings() {
        let mut keybinds = Keybinds::default();
        keybinds.view.flatten.back.clear();
        keybinds.view.flatten.back.insert(Event::Key(KeyEvent::new(
            KeyCode::Char('b'),
            KeyModifiers::CONTROL,
        )));
        let hints = QueryHints::flatten(&keybinds);

        assert_eq!(
            hints.create(HintFocus::Editor, QueryHintState::Ready),
            "focus: editor — Enter: execute — Esc: close editor — |: OR, &: AND"
        );
        assert_eq!(
            hints.create(HintFocus::Result, QueryHintState::Ready),
            "focus: result — Enter, LeftDown: goto — :, Shift+:: edit query — Ctrl+B: return"
        );
        assert_eq!(
            hints.create(HintFocus::Result, QueryHintState::Running),
            "focus: result — Ctrl+C: cancel — :, Shift+:: edit query — Ctrl+B: return"
        );
        assert_eq!(
            hints.create(HintFocus::Result, QueryHintState::Finished),
            "focus: result — Enter, LeftDown: goto — :, Shift+:: edit query — Ctrl+B: return"
        );
    }

    #[test]
    fn omits_disabled_actions_from_inline_hints() {
        let mut keybinds = Keybinds::default();
        keybinds.view.jaq.cancel_execution.clear();
        let hints = QueryHints::jaq(&keybinds);

        assert_eq!(
            hints.create(HintFocus::Editor, QueryHintState::Running),
            "focus: editor — Enter: execute — Ctrl+C, Esc: close editor"
        );
    }

    #[test]
    fn creates_help_hints_from_configured_bindings() {
        let mut keybinds = Keybinds::default();
        keybinds.view.help.back.clear();
        keybinds
            .view
            .help
            .back
            .insert(Event::Key(KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE)));

        assert_eq!(
            HelpHints::new(&keybinds).create(),
            "focus: content — use configured document movement keys — F1: return"
        );
    }
}
