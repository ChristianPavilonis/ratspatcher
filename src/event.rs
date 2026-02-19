use std::time::Duration;

use color_eyre::Result;
use crossterm::event::{self, Event as CrosstermEvent, KeyEvent, KeyEventKind};

/// Application events: either a terminal event or a periodic tick.
#[derive(Debug)]
pub enum Event {
    /// A key press event (only press, not release/repeat)
    Key(KeyEvent),
    /// A periodic tick for background updates
    Tick,
    /// Terminal resize
    Resize(u16, u16),
}

/// Polls for terminal events with a tick interval.
/// Returns `Some(Event)` if an event occurred, `None` if nothing happened.
pub fn poll_event(tick_rate: Duration) -> Result<Event> {
    if event::poll(tick_rate)? {
        match event::read()? {
            CrosstermEvent::Key(key) if key.kind == KeyEventKind::Press => Ok(Event::Key(key)),
            CrosstermEvent::Resize(w, h) => Ok(Event::Resize(w, h)),
            // Ignore other events (mouse, focus, key release/repeat)
            _ => Ok(Event::Tick),
        }
    } else {
        Ok(Event::Tick)
    }
}
