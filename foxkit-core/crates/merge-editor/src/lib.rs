//! # Foxkit Merge Editor
//!
//! 3-way merge conflict resolution editor.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

static CONFLICT_ID: AtomicU64 = AtomicU64::new(1);

/// Merge editor service
pub struct MergeEditorService {
    /// Active sessions
    sessions: RwLock<HashMap<PathBuf, MergeSession>>,
    /// Event sender
    event_tx: broadcast::Sender<MergeEvent>,
}

impl MergeEditorService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);

        Self {
            sessions: RwLock::new(HashMap::new()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<MergeEvent> {
        self.event_tx.subscribe()
    }

    /// Open merge session
    pub fn open_session(
        &self,
        file: PathBuf,
        base: String,
        ours: String,
        theirs: String,
    ) -> MergeSession {
        let session = MergeSession::new(file.clone(), base, ours, theirs);
        self.sessions.write().insert(file, session.clone());
        let _ = self.event_tx.send(MergeEvent::SessionOpened(session.clone()));
        session
    }

    /// Get session
    pub fn get_session(&self, file: &PathBuf) -> Option<MergeSession> {
        self.sessions.read().get(file).cloned()
    }

    /// Close session
    pub fn close_session(&self, file: &PathBuf) {
        if let Some(session) = self.sessions.write().remove(file) {
            let _ = self.event_tx.send(MergeEvent::SessionClosed(session.file));
        }
    }

    /// Accept current (ours)
    pub fn accept_current(&self, file: &PathBuf, conflict_id: &ConflictId) {
        if let Some(session) = self.sessions.write().get_mut(file) {
            if let Some(conflict) = session.conflicts.iter_mut().find(|c| &c.id == conflict_id) {
                conflict.resolution = Some(Resolution::Current);
                conflict.resolved_content = Some(conflict.current.clone());
                let _ = self.event_tx.send(MergeEvent::ConflictResolved {
                    file: file.clone(),
                    conflict_id: conflict_id.clone(),
                    resolution: Resolution::Current,
                });
            }
        }
    }

    /// Accept incoming (theirs)
    pub fn accept_incoming(&self, file: &PathBuf, conflict_id: &ConflictId) {
        if let Some(session) = self.sessions.write().get_mut(file) {
            if let Some(conflict) = session.conflicts.iter_mut().find(|c| &c.id == conflict_id) {
                conflict.resolution = Some(Resolution::Incoming);
                conflict.resolved_content = Some(conflict.incoming.clone());
                let _ = self.event_tx.send(MergeEvent::ConflictResolved {
                    file: file.clone(),
                    conflict_id: conflict_id.clone(),
                    resolution: Resolution::Incoming,
                });
            }
        }
    }

    /// Accept both
    pub fn accept_both(&self, file: &PathBuf, conflict_id: &ConflictId, current_first: bool) {
        if let Some(session) = self.sessions.write().get_mut(file) {
            if let Some(conflict) = session.conflicts.iter_mut().find(|c| &c.id == conflict_id) {
                let content = if current_first {
                    format!("{}\n{}", conflict.current, conflict.incoming)
                } else {
                    format!("{}\n{}", conflict.incoming, conflict.current)
                };
                conflict.resolution = Some(Resolution::Both);
                conflict.resolved_content = Some(content);
                let _ = self.event_tx.send(MergeEvent::ConflictResolved {
                    file: file.clone(),
                    conflict_id: conflict_id.clone(),
                    resolution: Resolution::Both,
                });
            }
        }
    }

    /// Accept base
    pub fn accept_base(&self, file: &PathBuf, conflict_id: &ConflictId) {
        if let Some(session) = self.sessions.write().get_mut(file) {
            if let Some(conflict) = session.conflicts.iter_mut().find(|c| &c.id == conflict_id) {
                conflict.resolution = Some(Resolution::Base);
                conflict.resolved_content = Some(conflict.base.clone());
                let _ = self.event_tx.send(MergeEvent::ConflictResolved {
                    file: file.clone(),
                    conflict_id: conflict_id.clone(),
                    resolution: Resolution::Base,
                });
            }
        }
    }

    /// Accept custom
    pub fn accept_custom(&self, file: &PathBuf, conflict_id: &ConflictId, content: String) {
        if let Some(session) = self.sessions.write().get_mut(file) {
            if let Some(conflict) = session.conflicts.iter_mut().find(|c| &c.id == conflict_id) {
                conflict.resolution = Some(Resolution::Custom);
                conflict.resolved_content = Some(content);
                let _ = self.event_tx.send(MergeEvent::ConflictResolved {
                    file: file.clone(),
                    conflict_id: conflict_id.clone(),
                    resolution: Resolution::Custom,
                });
            }
        }
    }

    /// Reset conflict resolution
    pub fn reset_conflict(&self, file: &PathBuf, conflict_id: &ConflictId) {
        if let Some(session) = self.sessions.write().get_mut(file) {
            if let Some(conflict) = session.conflicts.iter_mut().find(|c| &c.id == conflict_id) {
                conflict.resolution = None;
                conflict.resolved_content = None;
                let _ = self.event_tx.send(MergeEvent::ConflictReset {
                    file: file.clone(),
                    conflict_id: conflict_id.clone(),
                });
            }
        }
    }

    /// Get merged result
    pub fn get_merged_result(&self, file: &PathBuf) -> Option<MergeResult> {
        let sessions = self.sessions.read();
        let session = sessions.get(file)?;

        let unresolved: Vec<_> = session.conflicts
            .iter()
            .filter(|c| c.resolution.is_none())
            .cloned()
            .collect();

        if !unresolved.is_empty() {
            return Some(MergeResult::Incomplete { unresolved });
        }

        // Build merged content
        let merged = session.build_merged();
        Some(MergeResult::Complete { content: merged })
    }

    /// Navigate to next conflict
    pub fn next_conflict(&self, file: &PathBuf, current: Option<&ConflictId>) -> Option<ConflictId> {
        let sessions = self.sessions.read();
        let session = sessions.get(file)?;

        let unresolved: Vec<_> = session.conflicts
            .iter()
            .filter(|c| c.resolution.is_none())
            .collect();

        if unresolved.is_empty() {
            return None;
        }

        match current {
            None => unresolved.first().map(|c| c.id.clone()),
            Some(id) => {
                let pos = unresolved.iter().position(|c| &c.id == id);
                match pos {
                    Some(p) if p + 1 < unresolved.len() => {
                        Some(unresolved[p + 1].id.clone())
                    }
                    _ => unresolved.first().map(|c| c.id.clone()),
                }
            }
        }
    }

    /// Navigate to previous conflict
    pub fn prev_conflict(&self, file: &PathBuf, current: Option<&ConflictId>) -> Option<ConflictId> {
        let sessions = self.sessions.read();
        let session = sessions.get(file)?;

        let unresolved: Vec<_> = session.conflicts
            .iter()
            .filter(|c| c.resolution.is_none())
            .collect();

        if unresolved.is_empty() {
            return None;
        }

        match current {
            None => unresolved.last().map(|c| c.id.clone()),
            Some(id) => {
                let pos = unresolved.iter().position(|c| &c.id == id);
                match pos {
                    Some(0) => unresolved.last().map(|c| c.id.clone()),
                    Some(p) => Some(unresolved[p - 1].id.clone()),
                    None => unresolved.last().map(|c| c.id.clone()),
                }
            }
        }
    }

    /// Get conflict count
    pub fn conflict_count(&self, file: &PathBuf) -> (usize, usize) {
        let sessions = self.sessions.read();
        if let Some(session) = sessions.get(file) {
            let total = session.conflicts.len();
            let resolved = session.conflicts.iter().filter(|c| c.resolution.is_some()).count();
            (resolved, total)
        } else {
            (0, 0)
        }
    }
}

impl Default for MergeEditorService {
    fn default() -> Self {
        Self::new()
    }
}

/// Conflict ID
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConflictId(u64);

impl ConflictId {
    fn new() -> Self {
        Self(CONFLICT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Merge session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MergeSession {
    /// File path
    pub file: PathBuf,
    /// Base content
    pub base: String,
    /// Current (ours) content
    pub ours: String,
    /// Incoming (theirs) content
    pub theirs: String,
    /// Detected conflicts
    pub conflicts: Vec<Conflict>,
    /// Result content
    pub result: String,
}

impl MergeSession {
    pub fn new(file: PathBuf, base: String, ours: String, theirs: String) -> Self {
        let conflicts = Self::detect_conflicts(&base, &ours, &theirs);
        let result = ours.clone(); // Start with ours

        Self {
            file,
            base,
            ours,
            theirs,
            conflicts,
            result,
        }
    }

    fn detect_conflicts(base: &str, ours: &str, theirs: &str) -> Vec<Conflict> {
        // Simple conflict detection - in real impl would use diff3
        let mut conflicts = Vec::new();

        // Look for git conflict markers
        let lines: Vec<&str> = ours.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if lines[i].starts_with("<<<<<<<") {
                let start = i;
                let mut separator = None;
                let mut end = None;

                // Find separator and end
                for j in (i + 1)..lines.len() {
                    if lines[j].starts_with("=======") {
                        separator = Some(j);
                    } else if lines[j].starts_with(">>>>>>>") {
                        end = Some(j);
                        break;
                    }
                }

                if let (Some(sep), Some(e)) = (separator, end) {
                    let current_lines: Vec<_> = lines[(start + 1)..sep].to_vec();
                    let incoming_lines: Vec<_> = lines[(sep + 1)..e].to_vec();

                    conflicts.push(Conflict {
                        id: ConflictId::new(),
                        start_line: start as u32,
                        end_line: e as u32,
                        base: String::new(), // Would extract from base
                        current: current_lines.join("\n"),
                        incoming: incoming_lines.join("\n"),
                        resolution: None,
                        resolved_content: None,
                    });

                    i = e + 1;
                    continue;
                }
            }
            i += 1;
        }

        // If no markers found, check for differences
        if conflicts.is_empty() && ours != theirs {
            conflicts.push(Conflict {
                id: ConflictId::new(),
                start_line: 0,
                end_line: lines.len() as u32,
                base: base.to_string(),
                current: ours.to_string(),
                incoming: theirs.to_string(),
                resolution: None,
                resolved_content: None,
            });
        }

        conflicts
    }

    pub fn build_merged(&self) -> String {
        if self.conflicts.is_empty() {
            return self.ours.clone();
        }

        // Build from resolved conflicts
        let mut result = String::new();
        let lines: Vec<&str> = self.ours.lines().collect();
        let mut i = 0;

        for conflict in &self.conflicts {
            // Add lines before conflict
            while i < conflict.start_line as usize && i < lines.len() {
                result.push_str(lines[i]);
                result.push('\n');
                i += 1;
            }

            // Add resolved content
            if let Some(ref content) = conflict.resolved_content {
                result.push_str(content);
                if !content.ends_with('\n') {
                    result.push('\n');
                }
            }

            // Skip conflict lines
            i = (conflict.end_line + 1) as usize;
        }

        // Add remaining lines
        while i < lines.len() {
            result.push_str(lines[i]);
            result.push('\n');
            i += 1;
        }

        result
    }

    pub fn is_complete(&self) -> bool {
        self.conflicts.iter().all(|c| c.resolution.is_some())
    }

    pub fn unresolved_count(&self) -> usize {
        self.conflicts.iter().filter(|c| c.resolution.is_none()).count()
    }
}

/// Conflict
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conflict {
    /// Unique ID
    pub id: ConflictId,
    /// Start line
    pub start_line: u32,
    /// End line
    pub end_line: u32,
    /// Base content
    pub base: String,
    /// Current (ours) content
    pub current: String,
    /// Incoming (theirs) content
    pub incoming: String,
    /// Resolution choice
    pub resolution: Option<Resolution>,
    /// Resolved content
    pub resolved_content: Option<String>,
}

/// Resolution type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Resolution {
    Current,
    Incoming,
    Both,
    Base,
    Custom,
}

/// Merge result
#[derive(Debug, Clone)]
pub enum MergeResult {
    Complete { content: String },
    Incomplete { unresolved: Vec<Conflict> },
}

/// Merge event
#[derive(Debug, Clone)]
pub enum MergeEvent {
    SessionOpened(MergeSession),
    SessionClosed(PathBuf),
    ConflictResolved {
        file: PathBuf,
        conflict_id: ConflictId,
        resolution: Resolution,
    },
    ConflictReset {
        file: PathBuf,
        conflict_id: ConflictId,
    },
}

/// Word-level diff for conflict display
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordDiff {
    pub segments: Vec<WordDiffSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WordDiffSegment {
    pub text: String,
    pub kind: WordDiffKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum WordDiffKind {
    Equal,
    Added,
    Removed,
}
