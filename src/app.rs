use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use color_eyre::{eyre::eyre, Result};
use crossterm::event::KeyEvent;
use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Tabs},
    Frame,
};

use crate::action::Action;
use crate::components::add_repo::AddRepoModal;
use crate::components::dispatch_form::DispatchForm;
use crate::components::repo_list::RepoList;
use crate::components::run_list::RunList;
use crate::components::workflow_list::WorkflowList;
use crate::components::Component;
use crate::gh;
use crate::model::repo::Repo;
use crate::model::run::WorkflowRun;
use crate::model::workflow::Workflow;

/// Results from background gh CLI calls.
pub enum BgResult {
    WorkflowsLoaded(String, std::result::Result<Vec<Workflow>, String>),
    RunsLoaded(String, std::result::Result<Vec<WorkflowRun>, String>),
    DefaultBranchLoaded(String, String),
}

/// Which panel currently has focus.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    Sidebar,
    Main,
}

/// Which tab is active in the main panel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainTab {
    Workflows,
    Runs,
}

pub struct App {
    pub running: bool,
    pub focus: Focus,
    pub active_tab: MainTab,

    // Components
    pub repo_list: RepoList,
    pub workflow_list: WorkflowList,
    pub run_list: RunList,
    pub add_repo_modal: AddRepoModal,
    pub dispatch_form: DispatchForm,

    // Status bar message
    pub status_message: Option<String>,

    // Config path
    config_path: PathBuf,

    // Caches
    workflow_cache: HashMap<String, Vec<Workflow>>,
    run_cache: HashMap<String, Vec<WorkflowRun>>,
    default_branches: HashMap<String, String>,

    // Background task channel
    bg_tx: mpsc::Sender<BgResult>,
    bg_rx: mpsc::Receiver<BgResult>,

    // Track which repo is currently displayed so bg results land correctly
    current_repo: Option<String>,
}

impl App {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .ok_or_else(|| eyre!("Could not find config directory"))?
            .join("ratspatcher");
        std::fs::create_dir_all(&config_dir)?;
        let config_path = config_dir.join("repos.json");

        let (bg_tx, bg_rx) = mpsc::channel();

        let mut app = Self {
            running: true,
            focus: Focus::Sidebar,
            active_tab: MainTab::Workflows,
            repo_list: RepoList::new(),
            workflow_list: WorkflowList::new(),
            run_list: RunList::new(),
            add_repo_modal: AddRepoModal::new(),
            dispatch_form: DispatchForm::new(),
            status_message: None,
            config_path,
            workflow_cache: HashMap::new(),
            run_cache: HashMap::new(),
            default_branches: HashMap::new(),
            bg_tx,
            bg_rx,
            current_repo: None,
        };

        app.load_repos()?;
        Ok(app)
    }

    /// Load saved repos from config file.
    fn load_repos(&mut self) -> Result<()> {
        if self.config_path.exists() {
            let data = std::fs::read_to_string(&self.config_path)?;
            let repos: Vec<Repo> = serde_json::from_str(&data).unwrap_or_default();
            for repo in repos {
                self.repo_list.add_repo(repo);
            }
            // If we have repos, select the first and load its workflows
            if !self.repo_list.repos.is_empty() {
                self.repo_list.state.select(Some(0));
            }
        }
        Ok(())
    }

    /// Save repos to config file.
    fn save_repos(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.repo_list.repos)?;
        std::fs::write(&self.config_path, data)?;
        Ok(())
    }

    /// Process any completed background results. Call this every loop iteration.
    pub fn process_bg_results(&mut self) {
        while let Ok(result) = self.bg_rx.try_recv() {
            match result {
                BgResult::WorkflowsLoaded(repo_name, Ok(workflows)) => {
                    self.workflow_cache
                        .insert(repo_name.clone(), workflows.clone());
                    // Only update the UI if this result is for the currently selected repo
                    if self.current_repo.as_deref() == Some(&repo_name) {
                        self.workflow_list.set_workflows(workflows);
                    }
                }
                BgResult::WorkflowsLoaded(repo_name, Err(err)) => {
                    if self.current_repo.as_deref() == Some(&repo_name) {
                        self.workflow_list.set_error(err);
                    }
                }
                BgResult::RunsLoaded(repo_name, Ok(runs)) => {
                    self.run_cache.insert(repo_name.clone(), runs.clone());
                    if self.current_repo.as_deref() == Some(&repo_name) {
                        self.run_list.set_runs(runs);
                    }
                }
                BgResult::RunsLoaded(repo_name, Err(err)) => {
                    if self.current_repo.as_deref() == Some(&repo_name) {
                        self.run_list.set_error(err);
                    }
                }
                BgResult::DefaultBranchLoaded(repo_name, branch) => {
                    self.default_branches.insert(repo_name, branch);
                }
            }
        }
    }

    /// Handle a key event by dispatching to the appropriate component.
    pub fn handle_key_event(&mut self, key: KeyEvent) -> Action {
        // Modal takes priority
        if self.dispatch_form.visible {
            return self.dispatch_form.handle_key_event(key);
        }
        if self.add_repo_modal.visible {
            return self.add_repo_modal.handle_key_event(key);
        }

        // Route to focused component
        match self.focus {
            Focus::Sidebar => self.repo_list.handle_key_event(key),
            Focus::Main => match self.active_tab {
                MainTab::Workflows => self.workflow_list.handle_key_event(key),
                MainTab::Runs => self.run_list.handle_key_event(key),
            },
        }
    }

    /// Process an action and return any follow-up action.
    pub fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match &action {
            Action::Quit => {
                self.running = false;
            }
            Action::FocusSidebar => {
                self.focus = Focus::Sidebar;
                self.repo_list.focused = true;
                self.workflow_list.focused = false;
                self.run_list.focused = false;
            }
            Action::FocusMain => {
                self.focus = Focus::Main;
                self.repo_list.focused = false;
                match self.active_tab {
                    MainTab::Workflows => {
                        self.workflow_list.focused = true;
                        self.run_list.focused = false;
                    }
                    MainTab::Runs => {
                        self.workflow_list.focused = false;
                        self.run_list.focused = true;
                    }
                }
            }
            Action::OpenAddRepo => {
                self.add_repo_modal.open();
            }
            Action::CloseModal => {
                // Already handled by the modal itself
            }
            Action::AddRepo(name) => {
                match gh::validate_repo(name) {
                    Ok(repo) => {
                        let full_name = repo.full_name.clone();
                        self.repo_list.add_repo(repo);
                        let _ = self.save_repos();
                        self.add_repo_modal.close();
                        self.status_message = Some(format!("Added {}", full_name));
                        // Select the newly added repo and load workflows
                        let idx = self.repo_list.repos.len() - 1;
                        self.repo_list.state.select(Some(idx));
                        self.load_workflows_for_selected();
                        self.load_runs_for_selected();
                    }
                    Err(e) => {
                        self.add_repo_modal.loading = false;
                        self.add_repo_modal.error_msg = Some(format!("Not found: {}", e));
                    }
                }
            }
            Action::RemoveRepo => {
                if let Some(removed) = self.repo_list.remove_selected() {
                    // Clean up caches for removed repo
                    self.workflow_cache.remove(&removed.full_name);
                    self.run_cache.remove(&removed.full_name);
                    self.default_branches.remove(&removed.full_name);
                    let _ = self.save_repos();
                    self.status_message = Some(format!("Removed {}", removed.full_name));
                    self.workflow_list.clear();
                    self.run_list.clear();
                    // Load workflows for newly selected repo, if any
                    self.load_workflows_for_selected();
                    self.load_runs_for_selected();
                }
            }
            Action::SelectRepo(_) => {
                self.load_workflows_for_selected();
                self.load_runs_for_selected();
            }
            Action::TabWorkflows => {
                self.active_tab = MainTab::Workflows;
                self.workflow_list.focused = self.focus == Focus::Main;
                self.run_list.focused = false;
            }
            Action::TabRuns => {
                self.active_tab = MainTab::Runs;
                self.workflow_list.focused = false;
                self.run_list.focused = self.focus == Focus::Main;
            }
            Action::OpenDispatchForm(idx) => {
                if let Some(wf) = self.workflow_list.workflows.get(*idx).cloned() {
                    let default_branch = self.get_default_branch_for_selected();
                    self.dispatch_form.open(wf, *idx, &default_branch);
                }
            }
            Action::DispatchWorkflow(_, values) => {
                if let (Some(repo), Some(wf)) = (
                    self.repo_list.selected_repo(),
                    self.dispatch_form.workflow.as_ref(),
                ) {
                    let ref_name = self.dispatch_form.ref_input.value().to_string();
                    match gh::dispatch_workflow(&repo.full_name, &wf.name, &ref_name, values) {
                        Ok(()) => {
                            let wf_name = wf.name.clone();
                            let repo_name = repo.full_name.clone();
                            self.dispatch_form.close();
                            self.status_message = Some(format!("Dispatched '{}'", wf_name));
                            // Switch to runs tab
                            self.active_tab = MainTab::Runs;
                            self.run_list.focused = self.focus == Focus::Main;
                            self.workflow_list.focused = false;
                            // Invalidate run cache so we fetch fresh data
                            self.run_cache.remove(&repo_name);
                            self.load_runs_for_selected();
                        }
                        Err(e) => {
                            self.dispatch_form.dispatching = false;
                            self.dispatch_form.error_msg = Some(format!("Dispatch failed: {}", e));
                        }
                    }
                }
            }
            Action::Refresh => {
                // Invalidate caches for the current repo and reload
                if let Some(repo) = self.repo_list.selected_repo() {
                    let repo_name = repo.full_name.clone();
                    self.workflow_cache.remove(&repo_name);
                    self.run_cache.remove(&repo_name);
                    self.default_branches.remove(&repo_name);
                }
                self.load_workflows_for_selected();
                self.load_runs_for_selected();
                self.status_message = Some("Refreshing...".to_string());
            }
            Action::StatusMessage(msg) => {
                self.status_message = Some(msg.clone());
            }
            Action::Error(msg) => {
                self.status_message = Some(format!("Error: {}", msg));
            }
            _ => {}
        }

        // Propagate to focused components for navigation actions
        match &action {
            Action::Up | Action::Down => {
                if self.dispatch_form.visible {
                    let _ = self.dispatch_form.update(&action);
                } else if self.focus == Focus::Sidebar {
                    if let Ok(Some(follow_up)) = self.repo_list.update(&action) {
                        return self.update(follow_up);
                    }
                } else {
                    match self.active_tab {
                        MainTab::Workflows => {
                            let _ = self.workflow_list.update(&action);
                        }
                        MainTab::Runs => {
                            let _ = self.run_list.update(&action);
                        }
                    }
                }
            }
            Action::NextField | Action::PrevField => {
                if self.dispatch_form.visible {
                    let _ = self.dispatch_form.update(&action);
                }
            }
            _ => {}
        }

        Ok(None)
    }

    /// Load dispatchable workflows for the currently selected repo.
    /// Uses cache if available, otherwise spawns a background thread.
    fn load_workflows_for_selected(&mut self) {
        if let Some(repo) = self.repo_list.selected_repo() {
            let repo_name = repo.full_name.clone();
            self.current_repo = Some(repo_name.clone());

            // Check cache first
            if let Some(cached) = self.workflow_cache.get(&repo_name) {
                self.workflow_list.set_workflows(cached.clone());
                return;
            }

            // Cache miss — show loading and spawn background thread
            self.workflow_list.loading = true;
            let tx = self.bg_tx.clone();
            let name = repo_name.clone();
            std::thread::spawn(move || {
                let result = gh::load_dispatchable_workflows(&name).map_err(|e| e.to_string());
                let _ = tx.send(BgResult::WorkflowsLoaded(name, result));
            });
        } else {
            self.current_repo = None;
            self.workflow_list.clear();
        }
    }

    /// Load recent runs for the currently selected repo.
    /// Uses cache if available, otherwise spawns a background thread.
    fn load_runs_for_selected(&mut self) {
        if let Some(repo) = self.repo_list.selected_repo() {
            let repo_name = repo.full_name.clone();
            self.current_repo = Some(repo_name.clone());

            // Check cache first
            if let Some(cached) = self.run_cache.get(&repo_name) {
                self.run_list.set_runs(cached.clone());
                return;
            }

            // Cache miss — show loading and spawn background thread
            self.run_list.loading = true;
            let tx = self.bg_tx.clone();
            let name = repo_name.clone();
            std::thread::spawn(move || {
                let result = gh::list_runs(&name, None, 30).map_err(|e| e.to_string());
                let _ = tx.send(BgResult::RunsLoaded(name, result));
            });
        } else {
            self.current_repo = None;
            self.run_list.clear();
        }
    }

    /// Get the default branch for the currently selected repo (cached).
    /// Falls back to "main" if not yet loaded; also spawns a background
    /// fetch so the correct value is available next time.
    fn get_default_branch_for_selected(&mut self) -> String {
        if let Some(repo) = self.repo_list.selected_repo() {
            let name = repo.full_name.clone();
            if let Some(branch) = self.default_branches.get(&name) {
                return branch.clone();
            }
            // Spawn a background fetch so it's available next time
            let tx = self.bg_tx.clone();
            let bg_name = name.clone();
            std::thread::spawn(move || {
                let branch =
                    gh::get_default_branch(&bg_name).unwrap_or_else(|_| "main".to_string());
                let _ = tx.send(BgResult::DefaultBranchLoaded(bg_name, branch));
            });
            // Return "main" as a fallback for now
            "main".to_string()
        } else {
            "main".to_string()
        }
    }

    /// Render the full application UI.
    pub fn render(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Main layout: sidebar | content
        let horizontal =
            Layout::horizontal([Constraint::Length(30), Constraint::Min(40)]).split(area);

        // Content layout: tabs + content + status bar
        let vertical = Layout::vertical([
            Constraint::Length(3), // tabs
            Constraint::Min(1),    // content
            Constraint::Length(1), // status bar
        ])
        .split(horizontal[1]);

        // Render sidebar (repo list)
        self.repo_list.render(frame, horizontal[0]);

        // Render tabs
        let tab_titles = vec![Line::from(" Workflows "), Line::from(" Runs ")];
        let selected_tab = match self.active_tab {
            MainTab::Workflows => 0,
            MainTab::Runs => 1,
        };
        let tabs = Tabs::new(tab_titles)
            .select(selected_tab)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Tab to switch "),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .style(Style::default().fg(Color::DarkGray));
        frame.render_widget(tabs, vertical[0]);

        // Render active tab content
        match self.active_tab {
            MainTab::Workflows => self.workflow_list.render(frame, vertical[1]),
            MainTab::Runs => self.run_list.render(frame, vertical[1]),
        }

        // Render status bar
        let status_text = self
            .status_message
            .as_deref()
            .unwrap_or("q: quit | a: add repo | Tab: switch tab | r: refresh");
        let status = Paragraph::new(Line::from(vec![
            Span::styled(" ", Style::default()),
            Span::styled(status_text, Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(status, vertical[2]);

        // Render modals (on top of everything)
        self.add_repo_modal.render(frame, area);
        self.dispatch_form.render(frame, area);
    }
}
