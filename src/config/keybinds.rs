//! Configurable key assignments and their defaults. Runtime resolution belongs to the viewer.

use promkit::core::crossterm::event::Event;
use serde::Deserialize;
use std::collections::HashSet;
use termcfg::{crossterm_config::event_set_serde, event::parse::parse_shortcut};

mod entries;

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct Keybinds {
    pub(crate) component: ComponentKeybinds,
    pub(crate) view: ViewKeybinds,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct ComponentKeybinds {
    pub(crate) document: DocumentKeybinds,
    pub(crate) text_editor: TextEditorKeybinds,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
#[cfg_attr(test, schemars(transform = super::schema::legacy_jaq_alias))]
pub(crate) struct ViewKeybinds {
    pub(crate) browse: BrowseViewKeybinds,
    pub(crate) help: HelpViewKeybinds,
    pub(crate) preview: PreviewViewKeybinds,
    pub(crate) hstr: HstrViewKeybinds,
    pub(crate) flatten: FlattenViewKeybinds,
    #[serde(alias = "jq")]
    pub(crate) jaq: JaqViewKeybinds,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BrowseInputKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) exit: HashSet<Event>,
}

impl Default for BrowseInputKeybinds {
    fn default() -> Self {
        Self {
            exit: event_set(&["Esc", "Ctrl+C"]),
        }
    }
}

/// Navigation, commands, and search shared by input and focus views.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BrowseViewKeybinds {
    pub(crate) input: BrowseInputKeybinds,
    pub(crate) focus: BrowseFocusKeybinds,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_focus: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_side_by_side: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) open_command_editor: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) close_command_editor: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) submit_command: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) complete_command: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) open_search_editor: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) close_search_editor: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) submit_search: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) older_search_history: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) newer_search_history: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) next_search: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) previous_search: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) collapse_or_move_to_parent: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_parent: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) expand_or_move_to_first_child: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_next_sibling: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_previous_sibling: HashSet<Event>,
}

impl Default for BrowseViewKeybinds {
    fn default() -> Self {
        Self {
            input: BrowseInputKeybinds::default(),
            focus: BrowseFocusKeybinds::default(),
            toggle_focus: event_set(&["Tab"]),
            toggle_side_by_side: event_set(&["s"]),
            open_command_editor: event_set(&[":", "Shift+:"]),
            close_command_editor: event_set(&["Esc", "Ctrl+C"]),
            submit_command: event_set(&["Enter"]),
            complete_command: event_set(&["Tab"]),
            open_search_editor: event_set(&["/"]),
            close_search_editor: event_set(&["Esc", "Ctrl+C"]),
            submit_search: event_set(&["Enter"]),
            older_search_history: event_set(&["Up"]),
            newer_search_history: event_set(&["Down"]),
            next_search: event_set(&["n"]),
            previous_search: event_set(&["Shift+N"]),
            collapse_or_move_to_parent: event_set(&["h", "Left"]),
            move_to_parent: event_set(&["Shift+H"]),
            expand_or_move_to_first_child: event_set(&["l", "Right"]),
            move_to_next_sibling: event_set(&["Shift+J"]),
            move_to_previous_sibling: event_set(&["Shift+K"]),
        }
    }
}

/// Returning from a focused view has its own binding.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct BrowseFocusKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) back: HashSet<Event>,
}

impl Default for BrowseFocusKeybinds {
    fn default() -> Self {
        Self {
            back: event_set(&["Esc"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct HelpViewKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) back: HashSet<Event>,
}

impl Default for HelpViewKeybinds {
    fn default() -> Self {
        Self {
            back: event_set(&["Esc"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct PreviewViewKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) back: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) copy_selection: HashSet<Event>,
}

impl Default for PreviewViewKeybinds {
    fn default() -> Self {
        Self {
            back: event_set(&["Esc"]),
            copy_selection: event_set(&["y"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct HstrViewKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) back: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_focus: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) recall_selection: HashSet<Event>,
}

impl Default for HstrViewKeybinds {
    fn default() -> Self {
        Self {
            back: event_set(&["Esc"]),
            toggle_focus: event_set(&["Tab"]),
            recall_selection: event_set(&["Enter", "LeftDown"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct FlattenViewKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) back: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) cancel_execution: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) execute_query: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) open_query_editor: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) close_query_editor: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) goto_selection: HashSet<Event>,
}

impl Default for FlattenViewKeybinds {
    fn default() -> Self {
        Self {
            back: event_set(&["Esc"]),
            cancel_execution: event_set(&["Ctrl+C"]),
            execute_query: event_set(&["Enter"]),
            open_query_editor: event_set(&[":", "Shift+:"]),
            close_query_editor: event_set(&["Esc"]),
            goto_selection: event_set(&["Enter", "LeftDown"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct JaqViewKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) back: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) cancel_execution: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_focus: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_side_by_side: HashSet<Event>,
}

impl Default for JaqViewKeybinds {
    fn default() -> Self {
        Self {
            back: event_set(&["Esc"]),
            cancel_execution: event_set(&["Ctrl+C"]),
            toggle_focus: event_set(&["Tab"]),
            toggle_side_by_side: event_set(&["s"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct DocumentKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) up: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) down: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) page_up: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) page_down: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) half_page_up: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) half_page_down: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_head: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_tail: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) expand_all: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) collapse_all: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_overflow_mode: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) toggle_line_numbers: HashSet<Event>,
}

impl Default for DocumentKeybinds {
    fn default() -> Self {
        Self {
            up: event_set(&["k", "Up", "ScrollUp"]),
            down: event_set(&["j", "Down", "ScrollDown"]),
            page_up: event_set(&["PageUp", "Ctrl+B"]),
            page_down: event_set(&["PageDown", "Ctrl+F"]),
            half_page_up: event_set(&["Ctrl+U"]),
            half_page_down: event_set(&["Ctrl+D"]),
            move_to_head: event_set(&["g", "Home"]),
            move_to_tail: event_set(&["Shift+G", "End"]),
            toggle: event_set(&["Space", "Enter", "LeftDown"]),
            expand_all: event_set(&["e"]),
            collapse_all: event_set(&["Shift+E"]),
            toggle_overflow_mode: event_set(&["z"]),
            toggle_line_numbers: event_set(&["Alt+N"]),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[cfg_attr(test, derive(schemars::JsonSchema, serde::Serialize))]
#[serde(default, deny_unknown_fields)]
pub(crate) struct TextEditorKeybinds {
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) insert_newline: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) backward: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) forward: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_up: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_down: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_line_head: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_line_tail: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_head: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_tail: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_previous_nearest: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) move_to_next_nearest: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) erase: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) erase_forward: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) erase_all: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) erase_to_previous_nearest: HashSet<Event>,
    #[serde(with = "event_set_serde")]
    #[cfg_attr(test, schemars(with = "super::schema::KeyBinding"))]
    pub(crate) erase_to_next_nearest: HashSet<Event>,
}

impl Default for TextEditorKeybinds {
    fn default() -> Self {
        Self {
            insert_newline: event_set(&["Shift+Enter"]),
            backward: event_set(&["Left", "Ctrl+B"]),
            forward: event_set(&["Right", "Ctrl+F"]),
            move_up: event_set(&["Up"]),
            move_down: event_set(&["Down"]),
            move_to_line_head: event_set(&["Home", "Ctrl+A"]),
            move_to_line_tail: event_set(&["End", "Ctrl+E"]),
            move_to_head: event_set(&["Ctrl+Home"]),
            move_to_tail: event_set(&["Ctrl+End"]),
            move_to_previous_nearest: event_set(&["Alt+B", "Ctrl+Left"]),
            move_to_next_nearest: event_set(&["Alt+F", "Ctrl+Right"]),
            erase: event_set(&["Backspace", "Ctrl+H"]),
            erase_forward: event_set(&["Delete", "Ctrl+D"]),
            erase_all: event_set(&["Ctrl+U"]),
            erase_to_previous_nearest: event_set(&["Ctrl+W", "Alt+Backspace"]),
            erase_to_next_nearest: event_set(&["Alt+D", "Ctrl+Delete"]),
        }
    }
}

pub(crate) struct KeybindEntry<'a> {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) events: &'a HashSet<Event>,
}

fn event_set(shortcuts: &[&str]) -> HashSet<Event> {
    shortcuts
        .iter()
        .map(|shortcut| {
            parse_shortcut(shortcut)
                .expect("default shortcut must be valid")
                .into()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    use super::*;

    #[test]
    fn loads_custom_keybinds() {
        let config = Config::load_from(
            r#"
                [keybinds.view.browse.input]
                exit = "Ctrl+X"

                [keybinds.view.browse]
                open_command_editor = ";"
                move_to_parent = "p"

                [keybinds.component.document]
                up = ["u", "ScrollUp"]
                toggle = []

                [json]
                [yaml]
            "#,
        )
        .unwrap();

        assert_eq!(
            config.keybinds.view.browse.input.exit,
            event_set(&["Ctrl+X"])
        );
        assert_eq!(
            config.keybinds.view.browse.open_command_editor,
            event_set(&[";"])
        );
        assert_eq!(
            config.keybinds.view.browse.move_to_parent,
            event_set(&["p"])
        );
        assert_eq!(
            config.keybinds.view.browse.submit_command,
            Keybinds::default().view.browse.submit_command
        );
        assert_eq!(
            config.keybinds.component.document.up,
            event_set(&["u", "ScrollUp"])
        );
        assert!(config.keybinds.component.document.toggle.is_empty());
        assert_eq!(
            config.keybinds.component.document.down,
            Keybinds::default().component.document.down,
            "omitted operations retain their defaults"
        );
    }

    #[test]
    fn loads_legacy_jq_view_keybinds() {
        let config = Config::load_from(
            r#"
                [keybinds.view.jq]
                back = ["q"]

                [json]
                [yaml]
            "#,
        )
        .unwrap();

        assert_eq!(config.keybinds.view.jaq.back, event_set(&["q"]));
    }

    #[test]
    fn uses_default_keybinds_when_the_table_is_omitted() {
        let config = Config::load_from("[json]\n[yaml]").unwrap();

        assert_eq!(config.scroll_lines.get(), 3);
        assert_eq!(config.keybinds, Keybinds::default());
    }

    #[test]
    fn rejects_invalid_keybinds() {
        assert!(
            Config::load_from("[keybinds.view.browse.input]\nexit = \"NotAKey\"\n[json]\n[yaml]")
                .is_err()
        );
    }

    #[test]
    fn rejects_the_previous_flat_keybind_format() {
        assert!(Config::load_from("[keybinds]\nexit = \"Ctrl+X\"\n[json]\n[yaml]").is_err());
    }
}
