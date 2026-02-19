use serde::{Deserialize, Serialize};

/// A GitHub Actions workflow run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRun {
    #[serde(rename = "databaseId")]
    pub database_id: u64,
    pub name: String,
    #[serde(rename = "headBranch")]
    pub head_branch: String,
    pub status: String,
    pub conclusion: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "workflowName")]
    pub workflow_name: Option<String>,
}

impl WorkflowRun {
    /// Returns a display-friendly status string.
    pub fn display_status(&self) -> &str {
        if let Some(ref conclusion) = self.conclusion {
            conclusion.as_str()
        } else {
            self.status.as_str()
        }
    }

    /// Returns a symbol for the run status.
    pub fn status_symbol(&self) -> &str {
        match self.display_status() {
            "success" => "✓",
            "failure" => "✗",
            "cancelled" => "⊘",
            "in_progress" => "●",
            "queued" => "◌",
            "waiting" => "◌",
            _ => "?",
        }
    }
}
