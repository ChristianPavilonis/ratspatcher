mod action;
mod app;
mod components;
mod event;
mod gh;
mod model;
mod tui;

use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{KeyCode, KeyModifiers};

use crate::app::App;
use crate::event::{poll_event, Event};

fn main() -> Result<()> {
    color_eyre::install()?;

    let mut terminal = tui::init()?;
    let mut app = App::new()?;

    // If we have repos on startup, trigger initial load for the first one
    if app.repo_list.selected_repo().is_some() {
        let action = crate::action::Action::SelectRepo(0);
        let _ = app.update(action);
    }

    let tick_rate = Duration::from_millis(250);

    while app.running {
        // Process any completed background results
        app.process_bg_results();

        // Draw
        terminal.draw(|frame| {
            app.render(frame);
        })?;

        // Handle events
        match poll_event(tick_rate)? {
            Event::Key(key) => {
                // Ctrl+C always quits
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    app.running = false;
                    break;
                }
                let action = app.handle_key_event(key);
                if action != crate::action::Action::Noop {
                    let _ = app.update(action);
                }
            }
            Event::Tick => {
                // Background results are processed at the top of the loop
            }
            Event::Resize(_, _) => {
                // Terminal will handle resize on next draw
            }
        }
    }

    tui::restore()?;
    Ok(())
}
