use serde::{Deserialize, Serialize};

/// A GitHub repository identified by owner/name.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Repo {
    /// Full name in "owner/repo" format
    pub full_name: String,
}

impl Repo {
    pub fn new(full_name: impl Into<String>) -> Self {
        Self {
            full_name: full_name.into(),
        }
    }

    /// Returns the owner portion (before the slash).
    pub fn owner(&self) -> &str {
        self.full_name.split('/').next().unwrap_or("")
    }

    /// Returns the repo name portion (after the slash).
    pub fn name(&self) -> &str {
        self.full_name.split('/').nth(1).unwrap_or("")
    }
}

impl std::fmt::Display for Repo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.full_name)
    }
}
