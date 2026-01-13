//! SCM view models

use std::path::PathBuf;
use crate::{Change, ChangeKind, RepositoryState, BranchInfo, CommitInfo};

/// Source control view model
#[derive(Debug, Clone, Default)]
pub struct ScmViewModel {
    /// Current repository path
    pub repo_path: Option<PathBuf>,
    /// Repository state
    pub state: Option<RepositoryState>,
    /// View mode
    pub view_mode: ViewMode,
    /// Expanded groups
    pub expanded_groups: Vec<ChangeGroup>,
    /// Selected item
    pub selected: Option<PathBuf>,
    /// Commit message
    pub commit_message: String,
    /// Is committing
    pub is_committing: bool,
}

impl ScmViewModel {
    pub fn new() -> Self {
        Self {
            expanded_groups: vec![
                ChangeGroup::Staged,
                ChangeGroup::Changes,
            ],
            ..Default::default()
        }
    }

    /// Update from repository state
    pub fn update(&mut self, state: RepositoryState) {
        self.state = Some(state);
    }

    /// Get grouped changes
    pub fn grouped_changes(&self) -> Vec<ChangeTreeItem> {
        let Some(state) = &self.state else {
            return Vec::new();
        };

        let mut items = Vec::new();

        // Staged group
        let staged: Vec<_> = state.changes.iter()
            .filter(|c| c.staged)
            .collect();
        
        if !staged.is_empty() {
            items.push(ChangeTreeItem::Group {
                kind: ChangeGroup::Staged,
                count: staged.len(),
                expanded: self.expanded_groups.contains(&ChangeGroup::Staged),
                children: staged.iter()
                    .map(|c| ChangeTreeItem::Change((*c).clone()))
                    .collect(),
            });
        }

        // Changes group
        let changes: Vec<_> = state.changes.iter()
            .filter(|c| !c.staged && !matches!(c.kind, ChangeKind::Untracked))
            .collect();
        
        if !changes.is_empty() {
            items.push(ChangeTreeItem::Group {
                kind: ChangeGroup::Changes,
                count: changes.len(),
                expanded: self.expanded_groups.contains(&ChangeGroup::Changes),
                children: changes.iter()
                    .map(|c| ChangeTreeItem::Change((*c).clone()))
                    .collect(),
            });
        }

        // Untracked group
        let untracked: Vec<_> = state.changes.iter()
            .filter(|c| matches!(c.kind, ChangeKind::Untracked))
            .collect();
        
        if !untracked.is_empty() {
            items.push(ChangeTreeItem::Group {
                kind: ChangeGroup::Untracked,
                count: untracked.len(),
                expanded: self.expanded_groups.contains(&ChangeGroup::Untracked),
                children: untracked.iter()
                    .map(|c| ChangeTreeItem::Change((*c).clone()))
                    .collect(),
            });
        }

        items
    }

    /// Toggle group expansion
    pub fn toggle_group(&mut self, group: ChangeGroup) {
        if let Some(pos) = self.expanded_groups.iter().position(|g| *g == group) {
            self.expanded_groups.remove(pos);
        } else {
            self.expanded_groups.push(group);
        }
    }

    /// Get total change count
    pub fn change_count(&self) -> usize {
        self.state.as_ref()
            .map(|s| s.changes.len())
            .unwrap_or(0)
    }

    /// Can commit?
    pub fn can_commit(&self) -> bool {
        if self.commit_message.trim().is_empty() {
            return false;
        }
        
        self.state.as_ref()
            .map(|s| s.changes.iter().any(|c| c.staged))
            .unwrap_or(false)
    }
}

/// View mode
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ViewMode {
    #[default]
    Tree,
    List,
}

/// Change group
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeGroup {
    Staged,
    Changes,
    Untracked,
    Conflicts,
}

impl ChangeGroup {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Staged => "Staged Changes",
            Self::Changes => "Changes",
            Self::Untracked => "Untracked Files",
            Self::Conflicts => "Merge Conflicts",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::Staged => "✓",
            Self::Changes => "○",
            Self::Untracked => "?",
            Self::Conflicts => "!",
        }
    }
}

/// Change tree item
#[derive(Debug, Clone)]
pub enum ChangeTreeItem {
    Group {
        kind: ChangeGroup,
        count: usize,
        expanded: bool,
        children: Vec<ChangeTreeItem>,
    },
    Change(Change),
}

/// Branches view model
#[derive(Debug, Clone, Default)]
pub struct BranchesViewModel {
    /// All branches
    pub branches: Vec<BranchInfo>,
    /// Filter text
    pub filter: String,
    /// Show remote branches
    pub show_remote: bool,
    /// Selected branch
    pub selected: Option<String>,
}

impl BranchesViewModel {
    /// Get filtered branches
    pub fn filtered(&self) -> Vec<&BranchInfo> {
        self.branches.iter()
            .filter(|b| {
                if !self.show_remote && b.is_remote {
                    return false;
                }
                if !self.filter.is_empty() {
                    return b.name.to_lowercase()
                        .contains(&self.filter.to_lowercase());
                }
                true
            })
            .collect()
    }

    /// Get current branch
    pub fn current(&self) -> Option<&BranchInfo> {
        self.branches.iter().find(|b| b.is_current)
    }
}

/// History view model
#[derive(Debug, Clone, Default)]
pub struct HistoryViewModel {
    /// Commits
    pub commits: Vec<CommitInfo>,
    /// Selected commit
    pub selected: Option<String>,
    /// Is loading
    pub is_loading: bool,
    /// Has more commits
    pub has_more: bool,
    /// Filter by author
    pub author_filter: Option<String>,
    /// Filter by path
    pub path_filter: Option<PathBuf>,
}

impl HistoryViewModel {
    /// Get selected commit
    pub fn selected_commit(&self) -> Option<&CommitInfo> {
        self.selected.as_ref()
            .and_then(|id| self.commits.iter().find(|c| &c.id == id))
    }

    /// Load more commits
    pub fn can_load_more(&self) -> bool {
        !self.is_loading && self.has_more
    }
}

/// Commit message editor
#[derive(Debug, Clone, Default)]
pub struct CommitMessageEditor {
    /// Message text
    pub text: String,
    /// Cursor position
    pub cursor: usize,
    /// Template
    pub template: Option<String>,
    /// Is amending
    pub is_amend: bool,
    /// Previous commit message (for amend)
    pub previous_message: Option<String>,
}

impl CommitMessageEditor {
    /// Get message subject (first line)
    pub fn subject(&self) -> &str {
        self.text.lines().next().unwrap_or("")
    }

    /// Get message body
    pub fn body(&self) -> Option<&str> {
        self.text.split_once("\n\n").map(|(_, body)| body)
    }

    /// Is message valid?
    pub fn is_valid(&self) -> bool {
        let subject = self.subject();
        !subject.is_empty() && subject.len() <= 72
    }

    /// Get character count warning
    pub fn subject_warning(&self) -> Option<&'static str> {
        let len = self.subject().len();
        if len > 72 {
            Some("Subject line too long (>72 chars)")
        } else if len > 50 {
            Some("Subject line might be too long (>50 chars)")
        } else {
            None
        }
    }
}
