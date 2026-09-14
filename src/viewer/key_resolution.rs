//! Resolve events in explicit priority order for the active view context.

use promkit::core::crossterm::event::{Event, KeyEvent, KeyEventKind, MouseEvent};
use std::collections::HashSet;

use crate::config::Keybinds;

use super::{
    Action, BrowseFocus, BrowseViewAction, CommandAction, ComponentAction, DocumentAction,
    FlattenViewAction, HelpViewAction, HstrViewAction, JaqViewAction, PreviewViewAction,
    QueryFocus, TextEditorAction, ViewAction, ViewContext,
};

pub(super) fn resolve(keybinds: &Keybinds, context: ViewContext, event: &Event) -> Option<Action> {
    let event = normalize_event(event)?;

    match context {
        ViewContext::Input(BrowseFocus::Document) | ViewContext::Focus(BrowseFocus::Document) => {
            matches(
                if matches!(context, ViewContext::Focus(_)) {
                    &keybinds.view.browse.focus.back
                } else {
                    &keybinds.view.browse.input.exit
                },
                &event,
                Action::View(ViewAction::Browse(BrowseViewAction::Leave)),
            )
            .or_else(|| browse_document_action(keybinds, &event))
            .or_else(|| {
                matches!(context, ViewContext::Focus(_))
                    .then(|| {
                        matches(
                            &keybinds.view.browse.toggle_focus,
                            &event,
                            Action::View(ViewAction::Browse(BrowseViewAction::ToggleFocus)),
                        )
                    })
                    .flatten()
            })
            .or_else(|| {
                matches!(context, ViewContext::Focus(_))
                    .then(|| {
                        matches(
                            &keybinds.view.browse.toggle_side_by_side,
                            &event,
                            Action::View(ViewAction::Browse(BrowseViewAction::ToggleSideBySide)),
                        )
                    })
                    .flatten()
            })
            .or_else(|| document_action(keybinds, &event))
        }
        // The command editor uses the shared browse bindings in every hosting view.
        ViewContext::CommandEditor => matches(
            &keybinds.view.browse.close_command_editor,
            &event,
            Action::Component(ComponentAction::Command(CommandAction::Cancel)),
        )
        .or_else(|| {
            matches(
                &keybinds.view.browse.submit_command,
                &event,
                Action::Component(ComponentAction::Command(CommandAction::Submit)),
            )
        })
        .or_else(|| {
            matches(
                &keybinds.view.browse.complete_command,
                &event,
                Action::Component(ComponentAction::Command(CommandAction::Complete)),
            )
        })
        .or_else(|| text_editor_action(keybinds, &event)),
        ViewContext::Input(BrowseFocus::SearchEditor)
        | ViewContext::Focus(BrowseFocus::SearchEditor) => matches(
            &keybinds.view.browse.close_search_editor,
            &event,
            Action::View(ViewAction::Browse(BrowseViewAction::CloseSearchEditor)),
        )
        .or_else(|| {
            matches(
                &keybinds.view.browse.submit_search,
                &event,
                Action::View(ViewAction::Browse(BrowseViewAction::SubmitSearch)),
            )
        })
        .or_else(|| {
            matches(
                &keybinds.view.browse.older_search_history,
                &event,
                Action::View(ViewAction::Browse(BrowseViewAction::OlderSearchHistory)),
            )
        })
        .or_else(|| {
            matches(
                &keybinds.view.browse.newer_search_history,
                &event,
                Action::View(ViewAction::Browse(BrowseViewAction::NewerSearchHistory)),
            )
        })
        .or_else(|| text_editor_action(keybinds, &event)),
        ViewContext::Preview => matches(
            &keybinds.view.preview.back,
            &event,
            Action::View(ViewAction::Preview(PreviewViewAction::Back)),
        )
        .or_else(|| {
            matches(
                &keybinds.view.preview.copy_selection,
                &event,
                Action::View(ViewAction::Preview(PreviewViewAction::CopySelection)),
            )
        })
        .or_else(|| document_action(keybinds, &event)),
        ViewContext::Help => matches(
            &keybinds.view.help.back,
            &event,
            Action::View(ViewAction::Help(HelpViewAction::Back)),
        )
        .or_else(|| document_action(keybinds, &event)),
        ViewContext::Hstr(QueryFocus::Editor) => hstr_view_action(keybinds, &event)
            .or_else(|| {
                matches(
                    &keybinds.view.hstr.recall_selection,
                    &event,
                    Action::View(ViewAction::Hstr(HstrViewAction::RecallSelection)),
                )
            })
            .or_else(|| text_editor_action(keybinds, &event)),
        ViewContext::Hstr(QueryFocus::Result) => hstr_view_action(keybinds, &event)
            .or_else(|| {
                matches(
                    &keybinds.view.hstr.recall_selection,
                    &event,
                    Action::View(ViewAction::Hstr(HstrViewAction::RecallSelection)),
                )
            })
            .or_else(|| document_action(keybinds, &event)),
        ViewContext::Flatten(QueryFocus::Editor) => matches(
            &keybinds.view.flatten.close_query_editor,
            &event,
            Action::View(ViewAction::Flatten(FlattenViewAction::CloseQueryEditor)),
        )
        .or_else(|| flatten_view_action(keybinds, &event))
        .or_else(|| {
            matches(
                &keybinds.view.flatten.execute_query,
                &event,
                Action::View(ViewAction::Flatten(FlattenViewAction::ExecuteQuery)),
            )
        })
        .or_else(|| text_editor_action(keybinds, &event)),
        ViewContext::Flatten(QueryFocus::Result) => flatten_view_action(keybinds, &event)
            .or_else(|| {
                matches(
                    &keybinds.view.flatten.open_query_editor,
                    &event,
                    Action::View(ViewAction::Flatten(FlattenViewAction::OpenQueryEditor)),
                )
            })
            .or_else(|| {
                matches(
                    &keybinds.view.flatten.goto_selection,
                    &event,
                    Action::View(ViewAction::Flatten(FlattenViewAction::GotoSelection)),
                )
            })
            .or_else(|| document_action(keybinds, &event)),
        ViewContext::Jaq => matches(
            &keybinds.view.browse.open_command_editor,
            &event,
            Action::Component(ComponentAction::Command(CommandAction::Open)),
        )
        .or_else(|| jaq_view_action(keybinds, &event))
        .or_else(|| {
            matches(
                &keybinds.view.jaq.toggle_focus,
                &event,
                Action::View(ViewAction::Jaq(JaqViewAction::ToggleFocus)),
            )
        })
        .or_else(|| {
            matches(
                &keybinds.view.jaq.toggle_side_by_side,
                &event,
                Action::View(ViewAction::Jaq(JaqViewAction::ToggleSideBySide)),
            )
        })
        .or_else(|| browse_document_action(keybinds, &event))
        .or_else(|| document_action(keybinds, &event)),
    }
}

fn browse_document_action(keybinds: &Keybinds, event: &Event) -> Option<Action> {
    matches(
        &keybinds.view.browse.open_command_editor,
        event,
        Action::Component(ComponentAction::Command(CommandAction::Open)),
    )
    .or_else(|| {
        matches(
            &keybinds.view.browse.open_search_editor,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::OpenSearchEditor)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.next_search,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::NextSearch)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.previous_search,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::PreviousSearch)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.collapse_or_move_to_parent,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::CollapseOrMoveToParent)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.move_to_parent,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::MoveToParent)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.expand_or_move_to_first_child,
            event,
            Action::View(ViewAction::Browse(
                BrowseViewAction::ExpandOrMoveToFirstChild,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.move_to_next_sibling,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::MoveToNextSibling)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.view.browse.move_to_previous_sibling,
            event,
            Action::View(ViewAction::Browse(BrowseViewAction::MoveToPreviousSibling)),
        )
    })
}

fn matches(events: &HashSet<Event>, event: &Event, action: Action) -> Option<Action> {
    events.contains(event).then_some(action)
}

fn jaq_view_action(keybinds: &Keybinds, event: &Event) -> Option<Action> {
    matches(
        &keybinds.view.jaq.back,
        event,
        Action::View(ViewAction::Jaq(JaqViewAction::Back)),
    )
    .or_else(|| {
        matches(
            &keybinds.view.jaq.cancel_execution,
            event,
            Action::View(ViewAction::Jaq(JaqViewAction::CancelExecution)),
        )
    })
}

fn hstr_view_action(keybinds: &Keybinds, event: &Event) -> Option<Action> {
    matches(
        &keybinds.view.hstr.back,
        event,
        Action::View(ViewAction::Hstr(HstrViewAction::Back)),
    )
    .or_else(|| {
        matches(
            &keybinds.view.hstr.toggle_focus,
            event,
            Action::View(ViewAction::Hstr(HstrViewAction::ToggleFocus)),
        )
    })
}

fn flatten_view_action(keybinds: &Keybinds, event: &Event) -> Option<Action> {
    matches(
        &keybinds.view.flatten.back,
        event,
        Action::View(ViewAction::Flatten(FlattenViewAction::Back)),
    )
    .or_else(|| {
        matches(
            &keybinds.view.flatten.cancel_execution,
            event,
            Action::View(ViewAction::Flatten(FlattenViewAction::CancelExecution)),
        )
    })
}

fn document_action(keybinds: &Keybinds, event: &Event) -> Option<Action> {
    matches(
        &keybinds.component.document.up,
        event,
        Action::Component(ComponentAction::Document(DocumentAction::Up)),
    )
    .or_else(|| {
        matches(
            &keybinds.component.document.down,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::Down)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.page_up,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::PageUp)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.page_down,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::PageDown)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.half_page_up,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::HalfPageUp)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.half_page_down,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::HalfPageDown)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.move_to_head,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::MoveToHead)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.move_to_tail,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::MoveToTail)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.toggle,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::Toggle)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.expand_all,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::ExpandAll)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.collapse_all,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::CollapseAll)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.toggle_overflow_mode,
            event,
            Action::Component(ComponentAction::Document(
                DocumentAction::ToggleOverflowMode,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.document.toggle_line_numbers,
            event,
            Action::Component(ComponentAction::Document(DocumentAction::ToggleLineNumbers)),
        )
    })
}

fn text_editor_action(keybinds: &Keybinds, event: &Event) -> Option<Action> {
    matches(
        &keybinds.component.text_editor.insert_newline,
        event,
        Action::Component(ComponentAction::TextEditor(TextEditorAction::InsertNewline)),
    )
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.backward,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::Backward)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.forward,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::Forward)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_up,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::MoveUp)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_down,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::MoveDown)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_to_line_head,
            event,
            Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveToLineHead,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_to_line_tail,
            event,
            Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveToLineTail,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_to_head,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::MoveToHead)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_to_tail,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::MoveToTail)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_to_previous_nearest,
            event,
            Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveToPreviousNearest,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.move_to_next_nearest,
            event,
            Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveToNextNearest,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.erase,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::Erase)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.erase_forward,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::EraseForward)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.erase_all,
            event,
            Action::Component(ComponentAction::TextEditor(TextEditorAction::EraseAll)),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.erase_to_previous_nearest,
            event,
            Action::Component(ComponentAction::TextEditor(
                TextEditorAction::EraseToPreviousNearest,
            )),
        )
    })
    .or_else(|| {
        matches(
            &keybinds.component.text_editor.erase_to_next_nearest,
            event,
            Action::Component(ComponentAction::TextEditor(
                TextEditorAction::EraseToNextNearest,
            )),
        )
    })
}

fn normalize_event(event: &Event) -> Option<Event> {
    match event {
        Event::Key(key) if matches!(key.kind, KeyEventKind::Press | KeyEventKind::Repeat) => {
            Some(Event::Key(KeyEvent::new(key.code, key.modifiers)))
        }
        Event::Key(_) => None,
        Event::Mouse(mouse) => Some(Event::Mouse(MouseEvent {
            kind: mouse.kind,
            column: 0,
            row: 0,
            modifiers: mouse.modifiers,
        })),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use promkit::core::crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEventKind};

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    #[test]
    fn focus_uses_its_return_binding_and_shared_tree_and_editor_bindings() {
        let bindings: Keybinds = toml::from_str(
            r#"
[view.browse.input]
exit = ["x"]
[view.browse]
open_command_editor = [";"]
[view.browse.focus]
back = ["b"]
"#,
        )
        .unwrap();
        let context = ViewContext::Focus(BrowseFocus::Document);
        assert_eq!(
            resolve(
                &bindings,
                context,
                &key(KeyCode::Char('b'), KeyModifiers::NONE)
            ),
            Some(Action::View(ViewAction::Browse(BrowseViewAction::Leave)))
        );
        assert_eq!(
            resolve(
                &bindings,
                context,
                &key(KeyCode::Char('x'), KeyModifiers::NONE)
            ),
            None
        );
        assert_eq!(
            resolve(
                &bindings,
                context,
                &key(KeyCode::Char(';'), KeyModifiers::NONE)
            ),
            Some(Action::Component(ComponentAction::Command(
                CommandAction::Open
            )))
        );
        assert_eq!(
            resolve(
                &bindings,
                ViewContext::CommandEditor,
                &key(KeyCode::Esc, KeyModifiers::NONE)
            ),
            Some(Action::Component(ComponentAction::Command(
                CommandAction::Cancel
            )))
        );
        assert_eq!(
            resolve(
                &bindings,
                ViewContext::Focus(BrowseFocus::SearchEditor),
                &key(KeyCode::Esc, KeyModifiers::NONE)
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::CloseSearchEditor
            )))
        );
    }

    #[test]
    fn resolves_events_to_typed_actions_for_each_focus_context() {
        let keybinds = Keybinds::default();

        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char(':'), KeyModifiers::SHIFT),
            ),
            Some(Action::Component(ComponentAction::Command(
                CommandAction::Open
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::CommandEditor,
                &key(KeyCode::Tab, KeyModifiers::NONE),
            ),
            Some(Action::Component(ComponentAction::Command(
                CommandAction::Complete
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('/'), KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::OpenSearchEditor
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('n'), KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::NextSearch
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('N'), KeyModifiers::SHIFT),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::PreviousSearch
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::SearchEditor),
                &key(KeyCode::Enter, KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::SubmitSearch
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('h'), KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::CollapseOrMoveToParent
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('l'), KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::ExpandOrMoveToFirstChild
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('H'), KeyModifiers::SHIFT),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::MoveToParent
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('J'), KeyModifiers::SHIFT),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::MoveToNextSibling
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &key(KeyCode::Char('K'), KeyModifiers::SHIFT),
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::MoveToPreviousSibling
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Jaq,
                &key(KeyCode::Tab, KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Jaq(JaqViewAction::ToggleFocus)))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Jaq,
                &key(KeyCode::Char('s'), KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Jaq(
                JaqViewAction::ToggleSideBySide
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::CommandEditor,
                &key(KeyCode::Char('s'), KeyModifiers::NONE),
            ),
            None,
            "the comparison key remains available for query editing"
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Hstr(QueryFocus::Result),
                &key(KeyCode::Enter, KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Hstr(
                HstrViewAction::RecallSelection
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Hstr(QueryFocus::Editor),
                &key(KeyCode::Tab, KeyModifiers::NONE),
            ),
            Some(Action::View(ViewAction::Hstr(HstrViewAction::ToggleFocus)))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Jaq,
                &key(KeyCode::Char('j'), KeyModifiers::NONE),
            ),
            Some(Action::Component(ComponentAction::Document(
                DocumentAction::Down
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Jaq,
                &key(KeyCode::PageDown, KeyModifiers::NONE),
            ),
            Some(Action::Component(ComponentAction::Document(
                DocumentAction::PageDown
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::CommandEditor,
                &key(KeyCode::Enter, KeyModifiers::SHIFT),
            ),
            Some(Action::Component(ComponentAction::TextEditor(
                TextEditorAction::InsertNewline
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::CommandEditor,
                &key(KeyCode::Left, KeyModifiers::NONE),
            ),
            Some(Action::Component(ComponentAction::TextEditor(
                TextEditorAction::Backward
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::CommandEditor,
                &key(KeyCode::Char('b'), KeyModifiers::ALT),
            ),
            Some(Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveToPreviousNearest
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::CommandEditor,
                &key(KeyCode::Char('u'), KeyModifiers::CONTROL),
            ),
            Some(Action::Component(ComponentAction::TextEditor(
                TextEditorAction::EraseAll
            )))
        );

        let escape = key(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            resolve(&keybinds, ViewContext::CommandEditor, &escape),
            Some(Action::Component(ComponentAction::Command(
                CommandAction::Cancel
            )))
        );
        assert_eq!(
            resolve(&keybinds, ViewContext::Flatten(QueryFocus::Editor), &escape),
            Some(Action::View(ViewAction::Flatten(
                FlattenViewAction::CloseQueryEditor
            )))
        );

        let enter = key(KeyCode::Enter, KeyModifiers::NONE);
        assert_eq!(
            resolve(&keybinds, ViewContext::Flatten(QueryFocus::Editor), &enter),
            Some(Action::View(ViewAction::Flatten(
                FlattenViewAction::ExecuteQuery
            )))
        );
        assert_eq!(
            resolve(&keybinds, ViewContext::Flatten(QueryFocus::Result), &enter),
            Some(Action::View(ViewAction::Flatten(
                FlattenViewAction::GotoSelection
            )))
        );
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Flatten(QueryFocus::Result),
                &Event::Mouse(MouseEvent {
                    kind: MouseEventKind::Down(MouseButton::Left),
                    column: 42,
                    row: 24,
                    modifiers: KeyModifiers::NONE,
                }),
            ),
            Some(Action::View(ViewAction::Flatten(
                FlattenViewAction::GotoSelection
            )))
        );

        let tab = key(KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(
            resolve(&keybinds, ViewContext::Flatten(QueryFocus::Editor), &tab),
            None
        );
        assert_eq!(
            resolve(&keybinds, ViewContext::Flatten(QueryFocus::Result), &tab),
            None
        );

        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Flatten(QueryFocus::Editor),
                &key(KeyCode::Char('c'), KeyModifiers::CONTROL),
            ),
            Some(Action::View(ViewAction::Flatten(
                FlattenViewAction::CancelExecution
            )))
        );
    }

    #[test]
    fn colon_opens_editors_and_tab_completes_commands() {
        let keybinds = Keybinds::default();
        for (editor, result, open, back) in [
            (
                ViewContext::Flatten(QueryFocus::Editor),
                ViewContext::Flatten(QueryFocus::Result),
                Action::View(ViewAction::Flatten(FlattenViewAction::OpenQueryEditor)),
                ViewAction::Flatten(FlattenViewAction::Back),
            ),
            (
                ViewContext::CommandEditor,
                ViewContext::Jaq,
                Action::Component(ComponentAction::Command(CommandAction::Open)),
                ViewAction::Jaq(JaqViewAction::Back),
            ),
        ] {
            for modifiers in [KeyModifiers::NONE, KeyModifiers::SHIFT] {
                let colon = key(KeyCode::Char(':'), modifiers);
                assert_eq!(resolve(&keybinds, result, &colon), Some(open));
                assert_eq!(resolve(&keybinds, editor, &colon), None);
            }
            assert_eq!(
                resolve(&keybinds, editor, &key(KeyCode::Tab, KeyModifiers::NONE)),
                (editor == ViewContext::CommandEditor).then_some(Action::Component(
                    ComponentAction::Command(CommandAction::Complete)
                ))
            );
            assert_eq!(
                resolve(&keybinds, result, &key(KeyCode::Esc, KeyModifiers::NONE)),
                Some(Action::View(back))
            );
        }
    }

    #[test]
    fn jaq_has_no_separate_f2_editor_binding() {
        let keybinds = Keybinds::default();
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Jaq,
                &key(KeyCode::F(2), KeyModifiers::NONE)
            ),
            None
        );
    }

    #[test]
    fn resolves_the_same_event_according_to_its_destination() {
        let keybinds = Keybinds::default();
        let q = key(KeyCode::Char('q'), KeyModifiers::NONE);

        assert_eq!(
            resolve(&keybinds, ViewContext::CommandEditor, &q),
            None,
            "an unbound printable key is forwarded to the focused editor"
        );
        assert_eq!(
            resolve(&keybinds, ViewContext::Input(BrowseFocus::Document), &q),
            None
        );

        let scroll = Event::Mouse(MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 42,
            row: 24,
            modifiers: KeyModifiers::NONE,
        });
        assert_eq!(
            resolve(
                &keybinds,
                ViewContext::Input(BrowseFocus::Document),
                &scroll
            ),
            Some(Action::Component(ComponentAction::Document(
                DocumentAction::Down
            ))),
            "mouse coordinates do not affect keybinding resolution"
        );
    }
    #[test]
    fn search_history_keys_are_scoped_and_configurable() {
        let mut bindings = Keybinds::default();
        for context in [
            ViewContext::Input(BrowseFocus::SearchEditor),
            ViewContext::Focus(BrowseFocus::SearchEditor),
        ] {
            assert_eq!(
                resolve(&bindings, context, &key(KeyCode::Up, KeyModifiers::NONE)),
                Some(Action::View(ViewAction::Browse(
                    BrowseViewAction::OlderSearchHistory
                )))
            );
            assert_eq!(
                resolve(&bindings, context, &key(KeyCode::Down, KeyModifiers::NONE)),
                Some(Action::View(ViewAction::Browse(
                    BrowseViewAction::NewerSearchHistory
                )))
            );
        }
        assert_eq!(
            resolve(
                &bindings,
                ViewContext::CommandEditor,
                &key(KeyCode::Up, KeyModifiers::NONE)
            ),
            Some(Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveUp
            )))
        );
        bindings.view.browse.older_search_history =
            [key(KeyCode::Char('p'), KeyModifiers::CONTROL)]
                .into_iter()
                .collect();
        assert_eq!(
            resolve(
                &bindings,
                ViewContext::Input(BrowseFocus::SearchEditor),
                &key(KeyCode::Up, KeyModifiers::NONE)
            ),
            Some(Action::Component(ComponentAction::TextEditor(
                TextEditorAction::MoveUp
            )))
        );
        assert_eq!(
            resolve(
                &bindings,
                ViewContext::Input(BrowseFocus::SearchEditor),
                &key(KeyCode::Char('p'), KeyModifiers::CONTROL)
            ),
            Some(Action::View(ViewAction::Browse(
                BrowseViewAction::OlderSearchHistory
            )))
        );
    }
}
