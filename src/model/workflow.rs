use serde::{Deserialize, Serialize};

/// A GitHub Actions workflow that supports dispatch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    pub id: u64,
    pub name: String,
    pub path: String,
    pub state: String,
    /// Inputs defined under `on.workflow_dispatch.inputs`
    pub inputs: Vec<WorkflowInput>,
}

/// The type of a workflow dispatch input.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum InputType {
    String,
    Boolean,
    Choice,
    Environment,
}

/// A single input field for a workflow dispatch event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowInput {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub default: Option<String>,
    pub input_type: InputType,
    /// For choice inputs, the available options.
    pub options: Vec<String>,
}
