# ratspatcher

A terminal UI for dispatching GitHub Actions workflows. Browse repos, fill in workflow inputs, trigger runs, and monitor their status -- all without leaving the terminal.

Built with [ratatui](https://github.com/ratatui/ratatui) and powered by the [GitHub CLI](https://cli.github.com/).

## Features

- **Repo management** -- Add and remove GitHub repos from a persistent sidebar. Repos are saved to `~/.config/ratspatcher/repos.json`.
- **Workflow discovery** -- Automatically finds dispatchable workflows (`workflow_dispatch`) and parses their input definitions (string, boolean, choice, environment).
- **Dispatch form** -- Fill in branch/ref and all workflow inputs through a terminal form with validation.
- **Rerun last dispatch** -- Trigger the most recent successful workflow dispatch again with the same repo, ref, and inputs for the current app session.
- **Run monitoring** -- View the 30 most recent workflow runs in a color-coded table with status, branch, and timestamps.
- **Background loading** -- Workflow and run data loads in background threads so the UI stays responsive.

## Install

```bash
cargo install ratspatcher
```

### Prerequisites

The **GitHub CLI** (`gh`) must be installed and authenticated:

```bash
# install gh if needed
brew install gh       # macOS
# or see https://cli.github.com/

# authenticate
gh auth login
```

### Building from source

```bash
git clone https://github.com/ChristianPavilonis/ratspatcher.git
cd ratspatcher
cargo build --release
./target/release/ratspatcher
```

## Keybindings

### General

| Key | Action |
|-----|--------|
| `Ctrl+C` | Quit (always) |
| `q` | Quit (from sidebar) |
| `Tab` | Switch between Workflows / Runs tabs |
| `r` | Refresh data |
| `R` / `Shift+r` | Rerun the last successful workflow dispatch from this app session |

### Navigation

| Key | Action |
|-----|--------|
| `j` / `k` / `Up` / `Down` | Move up/down in lists |
| `h` / `Left` | Focus sidebar |
| `l` / `Right` / `Enter` | Focus main panel / select item |

### Sidebar

| Key | Action |
|-----|--------|
| `a` | Add a repo |
| `d` / `Delete` | Remove selected repo |

### Dispatch Form

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next / previous field |
| `Enter` | Advance field or dispatch |
| `Space` | Toggle boolean / dispatch |
| `Esc` | Cancel |

## Configuration

Repos are persisted at `~/.config/ratspatcher/repos.json` as a JSON array:

```json
[
  { "full_name": "owner/repo" }
]
```

No other configuration is needed. The app manages this file through the Add/Remove repo UI.

The last-dispatch rerun shortcut is kept in memory only for the current app session and is not written to configuration.
