pub mod add_repo;
pub mod dispatch_form;
pub mod repo_list;
pub mod run_list;
pub mod workflow_list;

use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::layout::Rect;
use ratatui::Frame;

use crate::action::Action;

/// Trait that all UI components implement.
/// Each component manages its own state, handles key events, and renders itself.
pub trait Component {
    /// Handle a key event and return an Action to be processed by the app.
    fn handle_key_event(&mut self, key: KeyEvent) -> Action;

    /// Update component state in response to an action.
    /// Returns a follow-up action if needed.
    fn update(&mut self, action: &Action) -> Result<Option<Action>> {
        let _ = action;
        Ok(None)
    }

    /// Render the component into the given area of the frame.
    fn render(&mut self, frame: &mut Frame, area: Rect);
}
