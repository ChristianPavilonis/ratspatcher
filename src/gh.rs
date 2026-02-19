use color_eyre::{eyre::eyre, Result};
use serde_json::Value;

use crate::model::repo::Repo;
use crate::model::run::WorkflowRun;
use crate::model::workflow::{InputType, Workflow, WorkflowInput};

/// Validate that a repo exists by calling `gh repo view`.
pub fn validate_repo(full_name: &str) -> Result<Repo> {
    let output = std::process::Command::new("gh")
        .args(["repo", "view", full_name, "--json", "nameWithOwner"])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!(
            "Failed to validate repo '{}': {}",
            full_name,
            stderr.trim()
        ));
    }

    let json: Value = serde_json::from_slice(&output.stdout)?;
    let name = json["nameWithOwner"]
        .as_str()
        .ok_or_else(|| eyre!("Missing nameWithOwner in response"))?;

    Ok(Repo::new(name))
}

/// List all workflows for a repo.
pub fn list_workflows(repo: &str) -> Result<Vec<Value>> {
    let output = std::process::Command::new("gh")
        .args([
            "workflow",
            "list",
            "--json",
            "id,name,path,state",
            "-R",
            repo,
        ])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("Failed to list workflows: {}", stderr.trim()));
    }

    let workflows: Vec<Value> = serde_json::from_slice(&output.stdout)?;
    Ok(workflows)
}

/// Fetch the raw YAML content of a workflow file.
pub fn get_workflow_yaml(repo: &str, workflow_name: &str) -> Result<String> {
    let output = std::process::Command::new("gh")
        .args(["workflow", "view", workflow_name, "--yaml", "-R", repo])
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("Failed to fetch workflow YAML: {}", stderr.trim()));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse a workflow YAML string and extract dispatch inputs.
/// Returns None if the workflow does not have a workflow_dispatch trigger.
pub fn parse_workflow_inputs(yaml_str: &str) -> Result<Option<Vec<WorkflowInput>>> {
    use serde_yaml::Value;

    let doc: Value = serde_yaml::from_str(yaml_str)
        .map_err(|e| eyre!("Failed to parse workflow YAML: {}", e))?;

    // The `on` key can be a string, list, or map.
    // In YAML, `on` is parsed as boolean `true` by serde_yaml, so check both.
    let on = match doc.get("on").or_else(|| doc.get(Value::Bool(true))) {
        Some(v) => v,
        None => return Ok(None),
    };

    // Check if workflow_dispatch is present
    let dispatch = match on {
        // on: workflow_dispatch
        Value::String(s) if s == "workflow_dispatch" => {
            return Ok(Some(vec![]));
        }
        // on: [push, workflow_dispatch]
        Value::Sequence(seq) => {
            let has_dispatch = seq
                .iter()
                .any(|v| v.as_str().is_some_and(|s| s == "workflow_dispatch"));
            if has_dispatch {
                return Ok(Some(vec![]));
            } else {
                return Ok(None);
            }
        }
        // on: { workflow_dispatch: { inputs: { ... } } }
        Value::Mapping(map) => match map.get(&Value::String("workflow_dispatch".to_string())) {
            Some(v) => v,
            None => return Ok(None),
        },
        _ => return Ok(None),
    };

    // If workflow_dispatch is null/empty, no inputs
    if on.is_null() {
        return Ok(Some(vec![]));
    }

    let inputs_map = match dispatch.get("inputs") {
        Some(Value::Mapping(map)) => map,
        _ => return Ok(Some(vec![])),
    };

    let mut inputs = Vec::new();
    for (key, value) in inputs_map {
        let name = key
            .as_str()
            .ok_or_else(|| eyre!("Input key is not a string"))?
            .to_string();

        let description = value
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let required = value
            .get("required")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let default = value.get("default").map(|v| match v {
            Value::String(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            other => format!("{:?}", other),
        });

        let type_str = value
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("string");

        let input_type = match type_str {
            "boolean" => InputType::Boolean,
            "choice" => InputType::Choice,
            "environment" => InputType::Environment,
            _ => InputType::String,
        };

        let options = value
            .get("options")
            .and_then(|v| v.as_sequence())
            .map(|seq| {
                seq.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        inputs.push(WorkflowInput {
            name,
            description,
            required,
            default,
            input_type,
            options,
        });
    }

    Ok(Some(inputs))
}

/// Load workflows for a repo, filtering to only dispatchable ones with parsed inputs.
pub fn load_dispatchable_workflows(repo: &str) -> Result<Vec<Workflow>> {
    let workflow_list = list_workflows(repo)?;
    let mut dispatchable = Vec::new();

    for wf in workflow_list {
        let id = wf["id"].as_u64().unwrap_or(0);
        let name = wf["name"].as_str().unwrap_or("").to_string();
        let path = wf["path"].as_str().unwrap_or("").to_string();
        let state = wf["state"].as_str().unwrap_or("").to_string();

        // Try to get the YAML and parse inputs
        match get_workflow_yaml(repo, &name) {
            Ok(yaml) => {
                match parse_workflow_inputs(&yaml) {
                    Ok(Some(inputs)) => {
                        dispatchable.push(Workflow {
                            id,
                            name,
                            path,
                            state,
                            inputs,
                        });
                    }
                    Ok(None) => {
                        // Not dispatchable, skip
                    }
                    Err(_) => {
                        // Failed to parse, skip
                    }
                }
            }
            Err(_) => {
                // Failed to fetch YAML, skip
            }
        }
    }

    Ok(dispatchable)
}

/// Dispatch a workflow with the given inputs.
pub fn dispatch_workflow(
    repo: &str,
    workflow_name: &str,
    ref_name: &str,
    inputs: &[(String, String)],
) -> Result<()> {
    let mut args = vec![
        "workflow".to_string(),
        "run".to_string(),
        workflow_name.to_string(),
        "-R".to_string(),
        repo.to_string(),
        "--ref".to_string(),
        ref_name.to_string(),
    ];

    for (key, value) in inputs {
        args.push("-f".to_string());
        args.push(format!("{}={}", key, value));
    }

    let output = std::process::Command::new("gh").args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("Failed to dispatch workflow: {}", stderr.trim()));
    }

    Ok(())
}

/// List recent workflow runs for a repo, optionally filtered by workflow name.
pub fn list_runs(
    repo: &str,
    workflow_name: Option<&str>,
    limit: usize,
) -> Result<Vec<WorkflowRun>> {
    let mut args = vec![
        "run".to_string(),
        "list".to_string(),
        "--json".to_string(),
        "databaseId,name,headBranch,status,conclusion,createdAt,workflowName".to_string(),
        "-R".to_string(),
        repo.to_string(),
        "-L".to_string(),
        limit.to_string(),
    ];

    if let Some(wf) = workflow_name {
        args.push("--workflow".to_string());
        args.push(wf.to_string());
    }

    let output = std::process::Command::new("gh").args(&args).output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("Failed to list runs: {}", stderr.trim()));
    }

    let runs: Vec<WorkflowRun> = serde_json::from_slice(&output.stdout)?;
    Ok(runs)
}

/// Get the default branch for a repo.
pub fn get_default_branch(repo: &str) -> Result<String> {
    let output = std::process::Command::new("gh")
        .args([
            "repo",
            "view",
            repo,
            "--json",
            "defaultBranchRef",
            "--jq",
            ".defaultBranchRef.name",
        ])
        .output()?;

    if !output.status.success() {
        return Ok("main".to_string());
    }

    let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if branch.is_empty() {
        Ok("main".to_string())
    } else {
        Ok(branch)
    }
}
