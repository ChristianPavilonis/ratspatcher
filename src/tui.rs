use std::io::{self, stdout, Stdout};

use color_eyre::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::CrosstermBackend, Terminal};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize the terminal: enable raw mode, enter alternate screen, set panic hook.
pub fn init() -> Result<Tui> {
    // Install a panic hook that restores the terminal before printing the panic
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        let _ = restore();
        original_hook(panic_info);
    }));

    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    let terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    Ok(terminal)
}

/// Restore the terminal to its original state.
pub fn restore() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;
    Ok(())
}
