//! Change tracking

use std::path::PathBuf;
use serde::{Deserialize, Serialize};

/// A changed file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    /// Absolute path
    pub path: PathBuf,
    /// Path relative to repo root
    pub relative_path: PathBuf,
    /// Change kind
    pub kind: ChangeKind,
    /// Is staged
    pub staged: bool,
    /// Original path (for renames)
    pub original_path: Option<PathBuf>,
}

impl Change {
    /// Get display name
    pub fn display_name(&self) -> String {
        self.relative_path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Get status icon
    pub fn icon(&self) -> &'static str {
        match self.kind {
            ChangeKind::Added => "A",
            ChangeKind::Modified => "M",
            ChangeKind::Deleted => "D",
            ChangeKind::Renamed => "R",
            ChangeKind::Copied => "C",
            ChangeKind::Untracked => "?",
            ChangeKind::Ignored => "!",
            ChangeKind::Conflicted => "U",
        }
    }

    /// Get status color
    pub fn color(&self) -> &'static str {
        match self.kind {
            ChangeKind::Added => "green",
            ChangeKind::Modified => "yellow",
            ChangeKind::Deleted => "red",
            ChangeKind::Renamed => "blue",
            ChangeKind::Copied => "blue",
            ChangeKind::Untracked => "gray",
            ChangeKind::Ignored => "gray",
            ChangeKind::Conflicted => "orange",
        }
    }
}

/// Change kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChangeKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
}

/// Resource state for SCM decoration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceState {
    /// No changes
    Clean,
    /// Has unstaged changes
    Modified,
    /// Has staged changes
    Staged,
    /// Has both staged and unstaged
    Mixed,
    /// Untracked file
    Untracked,
    /// Ignored file
    Ignored,
    /// Merge conflict
    Conflict,
    /// Added but not committed
    Added,
    /// Deleted but not committed
    Deleted,
}

impl ResourceState {
    /// Get decoration badge
    pub fn badge(&self) -> Option<&'static str> {
        match self {
            Self::Clean => None,
            Self::Modified => Some("M"),
            Self::Staged => Some("S"),
            Self::Mixed => Some("M"),
            Self::Untracked => Some("U"),
            Self::Ignored => Some("I"),
            Self::Conflict => Some("!"),
            Self::Added => Some("A"),
            Self::Deleted => Some("D"),
        }
    }

    /// Get decoration color
    pub fn color(&self) -> Option<&'static str> {
        match self {
            Self::Clean => None,
            Self::Modified => Some("#d19a66"),
            Self::Staged => Some("#98c379"),
            Self::Mixed => Some("#d19a66"),
            Self::Untracked => Some("#7f848e"),
            Self::Ignored => Some("#5c6370"),
            Self::Conflict => Some("#e06c75"),
            Self::Added => Some("#98c379"),
            Self::Deleted => Some("#e06c75"),
        }
    }
}

/// Group changes by state
pub fn group_changes(changes: &[Change]) -> ChangeGroups {
    let mut groups = ChangeGroups::default();

    for change in changes {
        if change.staged {
            groups.staged.push(change.clone());
        } else if matches!(change.kind, ChangeKind::Untracked) {
            groups.untracked.push(change.clone());
        } else if matches!(change.kind, ChangeKind::Conflicted) {
            groups.conflicts.push(change.clone());
        } else {
            groups.changes.push(change.clone());
        }
    }

    groups
}

/// Grouped changes
#[derive(Debug, Clone, Default)]
pub struct ChangeGroups {
    /// Staged changes
    pub staged: Vec<Change>,
    /// Unstaged changes
    pub changes: Vec<Change>,
    /// Untracked files
    pub untracked: Vec<Change>,
    /// Merge conflicts
    pub conflicts: Vec<Change>,
}

impl ChangeGroups {
    /// Total count
    pub fn total(&self) -> usize {
        self.staged.len() + self.changes.len() + self.untracked.len() + self.conflicts.len()
    }

    /// Is empty
    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}
