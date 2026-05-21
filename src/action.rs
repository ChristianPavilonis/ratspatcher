/// Actions represent messages passed between components and the app.
/// They decouple event handling from state mutations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// No-op, do nothing
    Noop,
    /// Quit the application
    Quit,
    /// Switch focus to the repo sidebar
    FocusSidebar,
    /// Switch focus to the main content panel
    FocusMain,
    /// Open the "Add Repo" modal
    OpenAddRepo,
    /// Close the current modal
    CloseModal,
    /// Confirm adding a repo with the given org/name
    AddRepo(String),
    /// Remove the currently selected repo
    RemoveRepo,
    /// Select a repo by index in the sidebar
    SelectRepo(usize),
    /// Switch to the Workflows tab
    TabWorkflows,
    /// Switch to the Runs tab
    TabRuns,
    /// Open the dispatch form for a workflow by index
    OpenDispatchForm(usize),
    /// Dispatch a workflow (workflow index, Vec of (key, value) pairs)
    DispatchWorkflow(usize, Vec<(String, String)>),
    /// Dispatch the last successful workflow again with the same repo/ref/inputs
    RerunLastWorkflow,
    /// Refresh the current view's data
    Refresh,
    /// Navigate up in a list
    Up,
    /// Navigate down in a list
    Down,
    /// Confirm / Enter
    Enter,
    /// Cycle to next form field
    NextField,
    /// Cycle to previous form field
    PrevField,
    /// Toggle a boolean field in the dispatch form
    ToggleField,
    /// Set a status message to display briefly
    StatusMessage(String),
    /// An error occurred
    Error(String),
}
