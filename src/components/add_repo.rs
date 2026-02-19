use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};
use tui_input::backend::crossterm::EventHandler;
use tui_input::Input;

use crate::action::Action;
use crate::components::Component;

pub struct AddRepoModal {
    pub input: Input,
    pub visible: bool,
    pub error_msg: Option<String>,
    pub loading: bool,
}

impl AddRepoModal {
    pub fn new() -> Self {
        Self {
            input: Input::default(),
            visible: false,
            error_msg: None,
            loading: false,
        }
    }

    pub fn open(&mut self) {
        self.visible = true;
        self.input.reset();
        self.error_msg = None;
        self.loading = false;
    }

    pub fn close(&mut self) {
        self.visible = false;
        self.input.reset();
        self.error_msg = None;
        self.loading = false;
    }
}

impl Component for AddRepoModal {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.visible || self.loading {
            return Action::Noop;
        }

        match key.code {
            KeyCode::Esc => {
                self.close();
                Action::CloseModal
            }
            KeyCode::Enter => {
                let value = self.input.value().trim().to_string();
                if value.is_empty() {
                    self.error_msg = Some("Please enter a repo name".to_string());
                    Action::Noop
                } else if !value.contains('/') {
                    self.error_msg = Some("Format: owner/repo".to_string());
                    Action::Noop
                } else {
                    self.loading = true;
                    self.error_msg = None;
                    Action::AddRepo(value)
                }
            }
            // Ctrl+C to close
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.close();
                Action::CloseModal
            }
            _ => {
                // Forward to tui-input
                self.input.handle_event(&crossterm::event::Event::Key(key));
                self.error_msg = None;
                Action::Noop
            }
        }
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        if !self.visible {
            return;
        }

        // Center the modal
        let modal_area = centered_rect(50, 7, area);

        // Clear the area behind the modal
        frame.render_widget(Clear, modal_area);

        let title = if self.loading {
            " Validating repo... "
        } else {
            " Add Repository (owner/repo) "
        };

        let border_color = if self.error_msg.is_some() {
            Color::Red
        } else {
            Color::Cyan
        };

        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let inner = block.inner(modal_area);
        frame.render_widget(block, modal_area);

        // Split inner area: input field + error message
        let chunks = Layout::vertical([
            Constraint::Length(1), // spacing
            Constraint::Length(1), // input
            Constraint::Length(1), // spacing
            Constraint::Length(1), // error
        ])
        .split(inner);

        // Render input
        let input_width = chunks[1].width as usize;
        let scroll = self.input.visual_scroll(input_width.saturating_sub(1));
        let input_paragraph = Paragraph::new(self.input.value())
            .style(Style::default().fg(Color::White))
            .scroll((0, scroll as u16));
        frame.render_widget(input_paragraph, chunks[1]);

        // Set cursor position
        if !self.loading {
            let cursor_x = chunks[1].x + (self.input.visual_cursor().saturating_sub(scroll) as u16);
            let cursor_y = chunks[1].y;
            frame.set_cursor_position((cursor_x, cursor_y));
        }

        // Render error if any
        if let Some(ref err) = self.error_msg {
            let err_paragraph = Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red));
            frame.render_widget(err_paragraph, chunks[3]);
        }
    }
}

/// Helper to create a centered rect of given percentage width and fixed height.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .split(area);
    let horizontal = Layout::horizontal([Constraint::Percentage(percent_x)])
        .flex(Flex::Center)
        .split(vertical[0]);
    horizontal[0]
}
