//! Viewer interfaces: input context, operations, notifications, effects, and rendered frames.
//! Concrete pages, features, and UI components share these types; execution lives in `runtime`.
//!
//! Each loop handles input through `View::update`, executes any requested effect, applies
//! `View::notify` to its recipients, and renders the active view. Notifications never request
//! further effects, and only ignored pointer input falls back to keybinding resolution.

use crate::{
    catalog::FlattenSubscription,
    command::{config::Config as ConfigCommand, copy::Target as CopyTarget},
    config::{Config, ConfigFile},
    document_source::DocumentSource,
};
use promkit::core::{
    ContentPosition, CreatedGraphemes, WidthMode,
    crossterm::event::{Event, MouseEventKind},
};

/// Feature components for vy-specific use cases.
///
/// This layer coordinates application-specific state and actions and may compose shared UI
/// components.
mod feature;

/// Shared UI components used as building blocks by views and feature components.
///
/// This layer encapsulates rendering and interaction logic that is not tied to one particular
/// screen or feature.
mod ui;

/// Full-screen pages that compose feature and shared UI components.
///
/// This layer owns screen-level layout, focus, event handling, and navigation effects.
mod view;

// Private implementation of the viewer itself, not additional UI layers.
mod key_resolution;
mod runtime;
mod stack;

pub(crate) use runtime::Viewer;
pub(crate) use ui::{DocumentComponent, DocumentDisplayConfig};

/// Keeps background loading visible for inspection in debug builds.
#[cfg(all(debug_assertions, not(test)))]
const DEBUG_LOADING_TICKS: usize = 10;
#[cfg(any(not(debug_assertions), test))]
const DEBUG_LOADING_TICKS: usize = 0;

pub(crate) enum ViewerOutcome {
    Exit,
    EditConfig,
    Print(DocumentSource),
}

pub(crate) struct ViewerSources {
    document: DocumentSource,
    flatten: FlattenSubscription,
    config_file: ConfigFile,
}

/// The input context used to resolve keybindings, not an identity for the concrete screen.
/// A view may select a different context while an editor is focused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum ViewContext {
    Input(BrowseFocus),
    Focus(BrowseFocus),
    Preview,
    CommandEditor,
    Help,
    Hstr(QueryFocus),
    Flatten(QueryFocus),
    Jaq,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum BrowseFocus {
    Document,
    SearchEditor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum QueryFocus {
    Editor,
    Result,
}

/// A terminal event resolved into an operation owned by a view or component.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum Action {
    View(ViewAction),
    Component(ComponentAction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum ViewAction {
    Browse(BrowseViewAction),
    Help(HelpViewAction),
    Preview(PreviewViewAction),
    Hstr(HstrViewAction),
    Flatten(FlattenViewAction),
    Jaq(JaqViewAction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum ComponentAction {
    Document(DocumentAction),
    TextEditor(TextEditorAction),
    Command(CommandAction),
}

/// Command operations shared by any view hosting a command session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum CommandAction {
    Open,
    Cancel,
    Submit,
    Complete,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum BrowseViewAction {
    ToggleFocus,
    ToggleSideBySide,
    Leave,
    OpenSearchEditor,
    CloseSearchEditor,
    SubmitSearch,
    OlderSearchHistory,
    NewerSearchHistory,
    NextSearch,
    PreviousSearch,
    CollapseOrMoveToParent,
    MoveToParent,
    ExpandOrMoveToFirstChild,
    MoveToNextSibling,
    MoveToPreviousSibling,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum HelpViewAction {
    Back,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum PreviewViewAction {
    Back,
    CopySelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum HstrViewAction {
    Back,
    ToggleFocus,
    RecallSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum FlattenViewAction {
    Back,
    CancelExecution,
    ExecuteQuery,
    OpenQueryEditor,
    CloseQueryEditor,
    GotoSelection,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum JaqViewAction {
    Back,
    CancelExecution,
    ToggleFocus,
    ToggleSideBySide,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum DocumentAction {
    Up,
    Down,
    PageUp,
    PageDown,
    HalfPageUp,
    HalfPageDown,
    MoveToHead,
    MoveToTail,
    Toggle,
    ExpandAll,
    CollapseAll,
    ToggleOverflowMode,
    ToggleLineNumbers,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::viewer) enum TextEditorAction {
    InsertNewline,
    Backward,
    Forward,
    MoveUp,
    MoveDown,
    MoveToLineHead,
    MoveToLineTail,
    MoveToHead,
    MoveToTail,
    MoveToPreviousNearest,
    MoveToNextNearest,
    Erase,
    EraseForward,
    EraseAll,
    EraseToPreviousNearest,
    EraseToNextNearest,
}

/// Pointer coordinates and movement amounts resolved by the viewer for the current viewport.
pub(in crate::viewer) struct ActionContext {
    pub(in crate::viewer) click_position: Option<ContentPosition>,
    pub(in crate::viewer) is_mouse: bool,
    pub(in crate::viewer) movement_lines: usize,
}

/// Input offered to a view. Execution results arrive separately through `ViewNotification`.
pub(in crate::viewer) enum ViewEvent<'a> {
    Action {
        action: Action,
        context: ActionContext,
    },
    Raw(&'a Event),
    /// Mouse coordinates mapped through the most recently rendered content pane.
    Pointer {
        kind: MouseEventKind,
        position: Option<ContentPosition>,
    },
    Tick,
}

/// Results and state changes applied by the viewer; notifications do not request effects.
pub(in crate::viewer) enum ViewNotification {
    Navigate { document_index: usize, path: String },
    RecallCommand(String),
    OpenSucceeded,
    OpenFailed(String),
    ConfigValue(String),
    ConfigTarget(Option<(String, String)>),
    ConfigUpdated(std::sync::Arc<ConfigUpdate>),
    ConfigFailed(String),
    CopySucceeded(String),
    CopyFailed(String),
    WriteSucceeded(String),
    WriteFailed(String),
}

pub(in crate::viewer) struct ConfigUpdate {
    pub(in crate::viewer) message: String,
    pub(in crate::viewer) config: Config,
}

pub(in crate::viewer) struct ComparisonSource {
    pub(in crate::viewer) source: DocumentSource,
    pub(in crate::viewer) origin: Option<(usize, String)>,
}

/// Describes a destination without constructing it; `Viewer` owns creation and navigation.
/// Display settings are boxed to keep input responses small; they are only needed when opening a view.
pub(in crate::viewer) enum ViewRequest {
    Config {
        content: String,
    },
    Flatten {
        data: FlattenSubscription,
    },
    /// `source` is already the extracted subtree; index/path identify its original location.
    Focus {
        comparison: Box<ComparisonSource>,
        source: DocumentSource,
        document_index: usize,
        path: String,
        display_config: Box<DocumentDisplayConfig>,
    },
    Preview {
        content: String,
        document_index: usize,
        path: String,
    },
    Help,
    Hstr,
    Jaq {
        query: String,
        source: DocumentSource,
        display_config: Box<DocumentDisplayConfig>,
    },
}

/// Input consumption and the work required before receiving the next event.
pub(in crate::viewer) enum ViewUpdate {
    /// The view does not handle this input; pointer dispatch may fall back to key resolution.
    Ignored,
    /// The input was consumed without changing the visible state.
    Handled,
    /// The input changed visible state and requires rendering.
    Render,
    /// Execute a request, notify its recipients, then render unless execution leaves the loop.
    Effect(ViewEffect),
}

/// A request executed by the viewer after input handling finishes.
pub(in crate::viewer) enum ViewEffect {
    Open(ViewRequest),
    Config(ConfigCommand),
    InspectConfig(String),
    Copy {
        target: CopyTarget,
        content: String,
    },
    Print(DocumentSource),
    Write {
        source: DocumentSource,
        destination: std::path::PathBuf,
    },
    Navigate {
        document_index: usize,
        path: String,
    },
    RecallCommand(String),
    Back,
    Exit,
}

#[derive(Default)]
pub(in crate::viewer) struct ViewFrame {
    pub(in crate::viewer) feedback: CreatedGraphemes,
    pub(in crate::viewer) content: CreatedGraphemes,
    pub(in crate::viewer) suggestions: CreatedGraphemes,
    pub(in crate::viewer) editor: CreatedGraphemes,
    pub(in crate::viewer) content_height: usize,
}

impl ViewFrame {
    pub(in crate::viewer) fn create(
        width: u16,
        height: u16,
        feedback: CreatedGraphemes,
        suggestions: CreatedGraphemes,
        editor: CreatedGraphemes,
        create_content: impl FnOnce(u16) -> CreatedGraphemes,
    ) -> Self {
        let content_height = height
            .saturating_sub(rendered_height(&feedback, width))
            .saturating_sub(rendered_height(&suggestions, width))
            .saturating_sub(rendered_height(&editor, width));
        let mut content = create_content(content_height);
        let content_height_limit = usize::from(content_height);
        content.layout.max_height = Some(
            content
                .layout
                .max_height
                .map_or(content_height_limit, |height| {
                    height.min(content_height_limit)
                }),
        );

        Self {
            feedback,
            content,
            suggestions,
            editor,
            content_height: usize::from(content_height.max(1)),
        }
    }
}

/// A standalone page that owns its rendering, focus, and input handling.
pub(in crate::viewer) trait View {
    fn context(&self) -> ViewContext;

    fn render(&mut self, width: u16, height: u16) -> ViewFrame;

    fn update(&mut self, input: ViewEvent<'_>) -> ViewUpdate;

    /// Applies a result or shared state change. The viewer schedules rendering afterwards.
    fn notify(&mut self, notification: ViewNotification);
}

fn rendered_height(created: &CreatedGraphemes, width: u16) -> u16 {
    if created.graphemes.is_empty() {
        return 0;
    }

    let width = usize::from(width.max(1));
    let height = created
        .graphemes
        .logical_lines()
        .iter()
        .map(|line| match created.layout.width_mode {
            WidthMode::Truncate => 1,
            WidthMode::Wrap => line.widths().max(1).div_ceil(width),
        })
        .sum::<usize>();
    let height = created
        .layout
        .max_height
        .map_or(height, |max_height| height.min(max_height));

    u16::try_from(height).unwrap_or(u16::MAX)
}

#[cfg(test)]
mod tests {
    use promkit::core::{WidgetLayout, grapheme::StyledGraphemes};

    use super::*;

    fn rows(count: usize) -> CreatedGraphemes {
        CreatedGraphemes {
            graphemes: StyledGraphemes::from_lines(
                (0..count).map(|index| StyledGraphemes::from(index.to_string())),
            ),
            ..Default::default()
        }
    }

    #[test]
    fn reserves_vertical_space_for_suggestions_and_editor() {
        let frame = ViewFrame::create(
            80,
            20,
            CreatedGraphemes::default(),
            rows(8),
            rows(1),
            |_| rows(20),
        );

        assert_eq!(frame.content.layout.max_height, Some(11));
        assert_eq!(frame.content_height, 11);
    }

    #[test]
    fn preserves_a_smaller_content_height_limit() {
        let frame = ViewFrame::create(
            80,
            20,
            CreatedGraphemes::default(),
            rows(8),
            rows(1),
            |_| CreatedGraphemes {
                layout: WidgetLayout {
                    max_height: Some(5),
                    ..Default::default()
                },
                ..rows(20)
            },
        );

        assert_eq!(frame.content.layout.max_height, Some(5));
    }
}
