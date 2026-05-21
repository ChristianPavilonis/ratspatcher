use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::action::Action;
use crate::components::Component;
use crate::model::workflow::Workflow;

pub struct WorkflowList {
    pub workflows: Vec<Workflow>,
    pub state: ListState,
    pub focused: bool,
    pub loading: bool,
    pub error: Option<String>,
}

impl WorkflowList {
    pub fn new() -> Self {
        Self {
            workflows: Vec::new(),
            state: ListState::default(),
            focused: false,
            loading: false,
            error: None,
        }
    }

    pub fn set_workflows(&mut self, workflows: Vec<Workflow>) {
        self.workflows = workflows;
        self.loading = false;
        self.error = None;
        if !self.workflows.is_empty() {
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
        self.workflows.clear();
        self.state.select(None);
        self.error = None;
        self.loading = false;
    }

    pub fn selected_workflow(&self) -> Option<&Workflow> {
        self.state.selected().and_then(|i| self.workflows.get(i))
    }
}

impl Component for WorkflowList {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.focused {
            return Action::Noop;
        }

        match key.code {
            KeyCode::Char('j') | KeyCode::Down => Action::Down,
            KeyCode::Char('k') | KeyCode::Up => Action::Up,
            KeyCode::Enter => {
                if let Some(idx) = self.state.selected() {
                    Action::OpenDispatchForm(idx)
                } else {
                    Action::Noop
                }
            }
            KeyCode::Char('h') | KeyCode::Left => Action::FocusSidebar,
            KeyCode::Tab => Action::TabRuns,
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
                    if selected + 1 < self.workflows.len() {
                        self.state.select(Some(selected + 1));
                    }
                } else if !self.workflows.is_empty() {
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
            .title(" Workflows (Enter to dispatch) ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        if self.loading {
            let paragraph = Paragraph::new("Loading workflows...")
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

        if self.workflows.is_empty() {
            let msg = "No dispatchable workflows found.\nSelect a repo from the sidebar.";
            let paragraph = Paragraph::new(msg)
                .style(Style::default().fg(Color::DarkGray))
                .block(block);
            frame.render_widget(paragraph, area);
            return;
        }

        let items: Vec<ListItem> = self
            .workflows
            .iter()
            .map(|wf| {
                let input_count = wf.inputs.len();
                let input_info = if input_count > 0 {
                    format!(" ({} inputs)", input_count)
                } else {
                    " (no inputs)".to_string()
                };
                let line = Line::from(vec![
                    Span::styled(&wf.name, Style::default().fg(Color::White)),
                    Span::styled(input_info, Style::default().fg(Color::DarkGray)),
                ]);
                ListItem::new(line)
            })
            .collect();

        let list = List::new(items)
            .block(block)
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("▶ ");

        frame.render_stateful_widget(list, area, &mut self.state);
    }
}
