use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};

use crate::action::Action;
use crate::components::Component;
use crate::model::run::WorkflowRun;

pub struct RunList {
    pub runs: Vec<WorkflowRun>,
    pub state: TableState,
    pub focused: bool,
    pub loading: bool,
    pub error: Option<String>,
}

impl RunList {
    pub fn new() -> Self {
        Self {
            runs: Vec::new(),
            state: TableState::default(),
            focused: false,
            loading: false,
            error: None,
        }
    }

    pub fn set_runs(&mut self, runs: Vec<WorkflowRun>) {
        self.runs = runs;
        self.loading = false;
        self.error = None;
        if !self.runs.is_empty() {
            self.state.select(Some(0));
        } else {
            self.state.select(None);
        }
    }

    pub fn set_error(&mut self, err: String) {
        self.error = Some(err);
        self.loading = false;
    }

    pub fn clear(&mut self) {
        self.runs.clear();
        self.state.select(None);
        self.error = None;
        self.loading = false;
    }
}

impl Component for RunList {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.focused {
            return Action::Noop;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Action::Down,
            KeyCode::Char('k') | KeyCode::Up => Action::Up,
            KeyCode::Char('h') | KeyCode::Left => Action::FocusSidebar,
            KeyCode::Tab => Action::TabWorkflows,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('r') => Action::Refresh,
            KeyCode::Char('R') => Action::RerunLastWorkflow,
            _ => Action::Noop,
        }
    }

    fn update(&mut self, action: &Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Up => {
                if let Some(selected) = self.state.selected() {
                    if selected > 0 {
                        self.state.select(Some(selected - 1));
                    }
                }
            }
            Action::Down => {
                if let Some(selected) = self.state.selected() {
                    if selected + 1 < self.runs.len() {
                        self.state.select(Some(selected + 1));
                    }
                } else if !self.runs.is_empty() {
                    self.state.select(Some(0));
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let border_color = if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let block = Block::default()
            .title(" Runs [r]efresh ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        if self.loading {
            let paragraph = Paragraph::new("Loading runs...")
                .style(Style::default().fg(Color::Yellow))
                .block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        if let Some(ref err) = self.error {
            let paragraph = Paragraph::new(format!("Error: {}", err))
                .style(Style::default().fg(Color::Red))
                .block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        if self.runs.is_empty() {
            let paragraph = Paragraph::new("No recent runs.")
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        let header = Row::new(vec![
            Cell::from("Status"),
            Cell::from("Workflow"),
            Cell::from("Branch"),
            Cell::from("Created"),
        ])
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        let rows: Vec<Row> = self
            .runs
            .iter()
            .map(|run| {
                let status_color = match run.display_status() {
                    "success" => Color::Green,
                    "failure" => Color::Red,
                    "cancelled" => Color::Yellow,
                    "in_progress" => Color::Blue,
                    "queued" | "waiting" => Color::DarkGray,
                    _ => Color::White,
                };

                let status_text = format!("{} {}", run.status_symbol(), run.display_status());

                Row::new(vec![
                    Cell::from(status_text).style(Style::default().fg(status_color)),
                    Cell::from(
                        run.workflow_name
                            .as_deref()
                            .unwrap_or(&run.name)
                            .to_string(),
                    ),
                    Cell::from(run.head_branch.clone()),
                    Cell::from(format_time(&run.created_at)),
                ])
            })
            .collect();

        let table = Table::new(
            rows,
            [
                Constraint::Length(16),
                Constraint::Percentage(35),
                Constraint::Percentage(25),
                Constraint::Percentage(25),
            ],
        )
        .header(header)
        .block(block)
        .row_highlight_style(
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        );

        frame.render_stateful_widget(table, area, &mut self.state);
    }
}

/// Format an ISO timestamp to a shorter display format.
fn format_time(iso: &str) -> String {
    // Simple: just take the date and time portion
    if iso.len() >= 16 {
        iso[..16].replace('T', " ")
    } else {
        iso.to_string()
    }
}
