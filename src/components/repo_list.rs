use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use crate::action::Action;
use crate::components::Component;
use crate::model::repo::Repo;

pub struct RepoList {
    pub repos: Vec<Repo>,
    pub state: ListState,
    pub focused: bool,
}

impl RepoList {
    pub fn new() -> Self {
        Self {
            repos: Vec::new(),
            state: ListState::default(),
            focused: true,
        }
    }

    pub fn selected_repo(&self) -> Option<&Repo> {
        self.state.selected().and_then(|i| self.repos.get(i))
    }

    pub fn add_repo(&mut self, repo: Repo) {
        // Don't add duplicates
        if !self.repos.iter().any(|r| r.full_name == repo.full_name) {
            self.repos.push(repo);
            if self.state.selected().is_none() {
                self.state.select(Some(0));
            }
        }
    }

    pub fn remove_selected(&mut self) -> Option<Repo> {
        if let Some(i) = self.state.selected() {
            if i < self.repos.len() {
                let removed = self.repos.remove(i);
                // Adjust selection
                if self.repos.is_empty() {
                    self.state.select(None);
                } else if i >= self.repos.len() {
                    self.state.select(Some(self.repos.len() - 1));
                }
                return Some(removed);
            }
        }
        None
    }
}

impl Component for RepoList {
    fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        if !self.focused {
            return Action::Noop;
        }

        match key.code {
            KeyCode::Char('a') => Action::OpenAddRepo,
            KeyCode::Char('d') | KeyCode::Delete => Action::RemoveRepo,
            KeyCode::Char('j') | KeyCode::Down => Action::Down,
            KeyCode::Char('k') | KeyCode::Up => Action::Up,
            KeyCode::Enter | KeyCode::Char('l') | KeyCode::Right => Action::FocusMain,
            KeyCode::Char('q') => Action::Quit,
            KeyCode::Char('r') => Action::Refresh,
            _ => Action::Noop,
        }
    }

    fn update(&mut self, action: &Action) -> color_eyre::Result<Option<Action>> {
        match action {
            Action::Up => {
                if let Some(selected) = self.state.selected() {
                    if selected > 0 {
                        self.state.select(Some(selected - 1));
                        return Ok(Some(Action::SelectRepo(selected - 1)));
                    }
                }
            }
            Action::Down => {
                if let Some(selected) = self.state.selected() {
                    if selected + 1 < self.repos.len() {
                        self.state.select(Some(selected + 1));
                        return Ok(Some(Action::SelectRepo(selected + 1)));
                    }
                } else if !self.repos.is_empty() {
                    self.state.select(Some(0));
                    return Ok(Some(Action::SelectRepo(0)));
                }
            }
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .repos
            .iter()
            .map(|repo| {
                let line = Line::from(vec![Span::raw(&repo.full_name)]);
                ListItem::new(line)
            })
            .collect();

        let border_color = if self.focused {
            Color::Cyan
        } else {
            Color::DarkGray
        };

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Repos [a]dd [d]elete ")
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(border_color)),
            )
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
