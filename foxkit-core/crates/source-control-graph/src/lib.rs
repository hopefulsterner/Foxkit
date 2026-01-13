//! # Foxkit Source Control Graph
//!
//! Git commit graph visualization.

use std::collections::HashMap;
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Source control graph service
pub struct SourceControlGraphService {
    /// Current graph
    graph: RwLock<Option<CommitGraph>>,
    /// Configuration
    config: RwLock<GraphConfig>,
    /// Event sender
    event_tx: broadcast::Sender<GraphEvent>,
}

impl SourceControlGraphService {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(64);

        Self {
            graph: RwLock::new(None),
            config: RwLock::new(GraphConfig::default()),
            event_tx,
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<GraphEvent> {
        self.event_tx.subscribe()
    }

    /// Load graph
    pub fn load_graph(&self, commits: Vec<Commit>, refs: Vec<GitRef>) {
        let graph = CommitGraph::build(commits, refs);
        *self.graph.write() = Some(graph.clone());
        let _ = self.event_tx.send(GraphEvent::Loaded(graph));
    }

    /// Get graph
    pub fn graph(&self) -> Option<CommitGraph> {
        self.graph.read().clone()
    }

    /// Get visible commits (paginated)
    pub fn visible_commits(&self, offset: usize, limit: usize) -> Vec<GraphRow> {
        self.graph.read()
            .as_ref()
            .map(|g| g.rows.iter().skip(offset).take(limit).cloned().collect())
            .unwrap_or_default()
    }

    /// Get commit by hash
    pub fn get_commit(&self, hash: &str) -> Option<Commit> {
        self.graph.read()
            .as_ref()
            .and_then(|g| g.commits.get(hash).cloned())
    }

    /// Get refs for commit
    pub fn get_refs(&self, hash: &str) -> Vec<GitRef> {
        self.graph.read()
            .as_ref()
            .map(|g| g.refs.iter().filter(|r| r.commit == hash).cloned().collect())
            .unwrap_or_default()
    }

    /// Search commits
    pub fn search(&self, query: &str) -> Vec<Commit> {
        let query = query.to_lowercase();
        
        self.graph.read()
            .as_ref()
            .map(|g| {
                g.commits.values()
                    .filter(|c| {
                        c.hash.starts_with(&query) ||
                        c.message.to_lowercase().contains(&query) ||
                        c.author.name.to_lowercase().contains(&query)
                    })
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Filter by branch
    pub fn filter_branch(&self, branch: &str) -> Vec<Commit> {
        // Would follow branch ancestry
        self.graph.read()
            .as_ref()
            .map(|g| {
                let branch_ref = g.refs.iter().find(|r| r.name == branch);
                if let Some(r) = branch_ref {
                    g.commits.get(&r.commit).cloned().into_iter().collect()
                } else {
                    Vec::new()
                }
            })
            .unwrap_or_default()
    }

    /// Configure
    pub fn configure(&self, config: GraphConfig) {
        *self.config.write() = config;
    }

    /// Get config
    pub fn config(&self) -> GraphConfig {
        self.config.read().clone()
    }

    /// Clear graph
    pub fn clear(&self) {
        *self.graph.write() = None;
        let _ = self.event_tx.send(GraphEvent::Cleared);
    }
}

impl Default for SourceControlGraphService {
    fn default() -> Self {
        Self::new()
    }
}

/// Commit graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommitGraph {
    /// Commits by hash
    pub commits: HashMap<String, Commit>,
    /// Refs (branches, tags)
    pub refs: Vec<GitRef>,
    /// Graph rows for rendering
    pub rows: Vec<GraphRow>,
    /// Lane count
    pub lane_count: usize,
}

impl CommitGraph {
    pub fn build(commits: Vec<Commit>, refs: Vec<GitRef>) -> Self {
        let mut commit_map = HashMap::new();
        for commit in &commits {
            commit_map.insert(commit.hash.clone(), commit.clone());
        }

        // Build graph rows
        let rows = Self::layout_graph(&commits);
        let lane_count = rows.iter().map(|r| r.lane + 1).max().unwrap_or(1);

        Self {
            commits: commit_map,
            refs,
            rows,
            lane_count,
        }
    }

    fn layout_graph(commits: &[Commit]) -> Vec<GraphRow> {
        let mut rows = Vec::new();
        let mut active_lanes: Vec<Option<String>> = Vec::new();

        for commit in commits {
            // Find or create lane for this commit
            let lane = active_lanes.iter()
                .position(|l| l.as_ref() == Some(&commit.hash))
                .unwrap_or_else(|| {
                    // Find first empty lane or add new one
                    active_lanes.iter()
                        .position(|l| l.is_none())
                        .unwrap_or_else(|| {
                            active_lanes.push(None);
                            active_lanes.len() - 1
                        })
                });

            // Build connections
            let mut connections = Vec::new();

            // Connect to parents
            for (i, parent) in commit.parents.iter().enumerate() {
                let parent_lane = if i == 0 {
                    // First parent continues in same lane
                    lane
                } else {
                    // Other parents get new lanes
                    active_lanes.iter()
                        .position(|l| l.is_none())
                        .unwrap_or_else(|| {
                            active_lanes.push(None);
                            active_lanes.len() - 1
                        })
                };

                connections.push(GraphConnection {
                    from_lane: lane,
                    to_lane: parent_lane,
                    kind: if i == 0 { ConnectionKind::Direct } else { ConnectionKind::Merge },
                });

                // Mark parent lane as active
                if parent_lane < active_lanes.len() {
                    active_lanes[parent_lane] = Some(parent.clone());
                }
            }

            // Clear current lane if no parents
            if commit.parents.is_empty() {
                active_lanes[lane] = None;
            }

            rows.push(GraphRow {
                commit_hash: commit.hash.clone(),
                lane,
                connections,
                active_lanes: active_lanes.iter().filter(|l| l.is_some()).count(),
            });
        }

        rows
    }
}

/// Graph row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphRow {
    /// Commit hash
    pub commit_hash: String,
    /// Lane index
    pub lane: usize,
    /// Connections to next row
    pub connections: Vec<GraphConnection>,
    /// Number of active lanes
    pub active_lanes: usize,
}

/// Graph connection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConnection {
    pub from_lane: usize,
    pub to_lane: usize,
    pub kind: ConnectionKind,
}

/// Connection kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ConnectionKind {
    Direct,
    Merge,
    Branch,
}

/// Commit
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// Full hash
    pub hash: String,
    /// Short hash
    pub short_hash: String,
    /// Parent hashes
    pub parents: Vec<String>,
    /// Author
    pub author: Person,
    /// Committer
    pub committer: Person,
    /// Commit message
    pub message: String,
    /// Summary (first line)
    pub summary: String,
    /// Commit timestamp
    pub timestamp: DateTime<Utc>,
}

impl Commit {
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    pub fn is_root(&self) -> bool {
        self.parents.is_empty()
    }
}

/// Person (author/committer)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Person {
    pub name: String,
    pub email: String,
}

/// Git ref (branch/tag)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRef {
    /// Ref name
    pub name: String,
    /// Full ref path
    pub full_name: String,
    /// Commit hash
    pub commit: String,
    /// Ref type
    pub kind: RefKind,
    /// Is current HEAD
    pub is_head: bool,
    /// Remote name (for remote branches)
    pub remote: Option<String>,
}

impl GitRef {
    pub fn display_name(&self) -> &str {
        if let Some(remote) = &self.remote {
            self.name.strip_prefix(&format!("{}/", remote)).unwrap_or(&self.name)
        } else {
            &self.name
        }
    }
}

/// Ref kind
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
    Head,
    Stash,
}

impl RefKind {
    pub fn color(&self) -> &'static str {
        match self {
            Self::LocalBranch => "gitDecoration.branchLocalForeground",
            Self::RemoteBranch => "gitDecoration.branchRemoteForeground",
            Self::Tag => "gitDecoration.tagForeground",
            Self::Head => "gitDecoration.headForeground",
            Self::Stash => "gitDecoration.stashForeground",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            Self::LocalBranch => "git-branch",
            Self::RemoteBranch => "cloud",
            Self::Tag => "tag",
            Self::Head => "circle-filled",
            Self::Stash => "archive",
        }
    }
}

/// Graph configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphConfig {
    /// Show remote branches
    pub show_remotes: bool,
    /// Show tags
    pub show_tags: bool,
    /// Show stashes
    pub show_stashes: bool,
    /// Color scheme
    pub color_scheme: ColorScheme,
    /// Max commits to load
    pub max_commits: usize,
    /// Show author avatars
    pub show_avatars: bool,
    /// Date format
    pub date_format: DateFormat,
}

impl Default for GraphConfig {
    fn default() -> Self {
        Self {
            show_remotes: true,
            show_tags: true,
            show_stashes: false,
            color_scheme: ColorScheme::Branch,
            max_commits: 1000,
            show_avatars: true,
            date_format: DateFormat::Relative,
        }
    }
}

/// Color scheme
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ColorScheme {
    /// Color by branch
    Branch,
    /// Color by author
    Author,
    /// Single color
    Mono,
}

/// Date format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DateFormat {
    Relative,
    Absolute,
    Short,
    Iso,
}

/// Graph event
#[derive(Debug, Clone)]
pub enum GraphEvent {
    Loaded(CommitGraph),
    Cleared,
}

/// Lane colors
pub const LANE_COLORS: &[&str] = &[
    "#f14c4c", // red
    "#cca700", // yellow
    "#3dc9b0", // cyan
    "#6e6cd2", // purple
    "#ee9d28", // orange
    "#6a9955", // green
    "#569cd6", // blue
    "#c586c0", // magenta
];

pub fn lane_color(lane: usize) -> &'static str {
    LANE_COLORS[lane % LANE_COLORS.len()]
}
