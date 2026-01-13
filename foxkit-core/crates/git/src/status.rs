//! Git status

use std::path::{Path, PathBuf};
use crate::ChangeKind;

/// Repository status
#[derive(Debug, Clone, Default)]
pub struct Status {
    pub entries: Vec<FileStatus>,
}

impl Status {
    pub fn new() -> Self {
        Self::default()
    }

    /// Get staged files
    pub fn staged(&self) -> impl Iterator<Item = &FileStatus> {
        self.entries.iter().filter(|e| e.is_staged())
    }

    /// Get unstaged files
    pub fn unstaged(&self) -> impl Iterator<Item = &FileStatus> {
        self.entries.iter().filter(|e| !e.is_staged() && e.workdir != StatusKind::Unchanged)
    }

    /// Get untracked files
    pub fn untracked(&self) -> impl Iterator<Item = &FileStatus> {
        self.entries.iter().filter(|e| e.workdir == StatusKind::Untracked)
    }

    /// Check if working directory is clean
    pub fn is_clean(&self) -> bool {
        self.entries.iter().all(|e| {
            e.index == StatusKind::Unchanged && e.workdir == StatusKind::Unchanged
        })
    }

    /// Count of changes
    pub fn change_count(&self) -> usize {
        self.entries.iter().filter(|e| {
            e.index != StatusKind::Unchanged || e.workdir != StatusKind::Unchanged
        }).count()
    }
}

/// Status of a single file
#[derive(Debug, Clone)]
pub struct FileStatus {
    pub path: PathBuf,
    pub index: StatusKind,
    pub workdir: StatusKind,
    pub old_path: Option<PathBuf>,
}

impl FileStatus {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            index: StatusKind::Unchanged,
            workdir: StatusKind::Unchanged,
            old_path: None,
        }
    }

    /// Is this file staged?
    pub fn is_staged(&self) -> bool {
        self.index != StatusKind::Unchanged
    }

    /// Is this file modified in working directory?
    pub fn is_modified(&self) -> bool {
        self.workdir == StatusKind::Modified
    }

    /// Get the change kind for display
    pub fn change_kind(&self) -> ChangeKind {
        if self.index != StatusKind::Unchanged {
            self.index.to_change_kind()
        } else {
            self.workdir.to_change_kind()
        }
    }
}

/// Status kind
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    Unchanged,
    Added,
    Modified,
    Deleted,
    Renamed,
    Copied,
    Untracked,
    Ignored,
    Conflicted,
}

impl StatusKind {
    pub fn to_change_kind(self) -> ChangeKind {
        match self {
            StatusKind::Unchanged => ChangeKind::Modified, // Shouldn't happen
            StatusKind::Added => ChangeKind::Added,
            StatusKind::Modified => ChangeKind::Modified,
            StatusKind::Deleted => ChangeKind::Deleted,
            StatusKind::Renamed => ChangeKind::Renamed,
            StatusKind::Copied => ChangeKind::Copied,
            StatusKind::Untracked => ChangeKind::Untracked,
            StatusKind::Ignored => ChangeKind::Ignored,
            StatusKind::Conflicted => ChangeKind::Conflicted,
        }
    }
}

impl Default for StatusKind {
    fn default() -> Self {
        Self::Unchanged
    }
}
