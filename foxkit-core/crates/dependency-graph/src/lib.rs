//! Dependency graph analysis for monorepos.
//!
//! This crate provides tools for analyzing package dependencies,
//! detecting cycles, and understanding the structure of monorepos.

pub mod build_order;

pub use build_order::{BuildOrderComputer, BuildPlan, BuildStage, BuildNode, IncrementalBuild};

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

/// Unique identifier for a package node.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PackageId(pub String);

impl PackageId {
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }
}

/// Package node in the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageNode {
    pub id: PackageId,
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub package_type: PackageType,
}

/// Type of package.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageType {
    Library,
    Binary,
    Application,
    Test,
    Example,
}

/// Dependency edge.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: PackageId,
    pub to: PackageId,
    pub dependency_type: DependencyType,
    pub version_req: Option<String>,
}

/// Type of dependency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DependencyType {
    Normal,
    Dev,
    Build,
    Peer,
    Optional,
}

/// Dependency cycle detected in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyCycle {
    pub packages: Vec<PackageId>,
}

/// Analysis result for the dependency graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphAnalysis {
    pub total_packages: usize,
    pub total_dependencies: usize,
    pub root_packages: Vec<PackageId>,
    pub leaf_packages: Vec<PackageId>,
    pub cycles: Vec<DependencyCycle>,
    pub max_depth: usize,
}

/// The dependency graph structure.
pub struct DependencyGraph {
    nodes: RwLock<HashMap<PackageId, PackageNode>>,
    edges: RwLock<Vec<DependencyEdge>>,
    adjacency: RwLock<HashMap<PackageId, Vec<PackageId>>>,
    reverse_adjacency: RwLock<HashMap<PackageId, Vec<PackageId>>>,
}

impl DependencyGraph {
    /// Create a new empty dependency graph.
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(HashMap::new()),
            edges: RwLock::new(Vec::new()),
            adjacency: RwLock::new(HashMap::new()),
            reverse_adjacency: RwLock::new(HashMap::new()),
        }
    }

    /// Add a package node.
    pub fn add_package(&self, node: PackageNode) {
        let id = node.id.clone();
        self.nodes.write().insert(id.clone(), node);
        self.adjacency.write().entry(id.clone()).or_default();
        self.reverse_adjacency.write().entry(id).or_default();
    }

    /// Add a dependency edge.
    pub fn add_dependency(&self, edge: DependencyEdge) {
        self.adjacency
            .write()
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
        self.reverse_adjacency
            .write()
            .entry(edge.to.clone())
            .or_default()
            .push(edge.from.clone());
        self.edges.write().push(edge);
    }

    /// Get a package by ID.
    pub fn get_package(&self, id: &PackageId) -> Option<PackageNode> {
        self.nodes.read().get(id).cloned()
    }

    /// Get direct dependencies of a package.
    pub fn get_dependencies(&self, id: &PackageId) -> Vec<PackageId> {
        self.adjacency.read().get(id).cloned().unwrap_or_default()
    }

    /// Get packages that depend on this package.
    pub fn get_dependents(&self, id: &PackageId) -> Vec<PackageId> {
        self.reverse_adjacency.read().get(id).cloned().unwrap_or_default()
    }

    /// Get all transitive dependencies.
    pub fn get_transitive_dependencies(&self, id: &PackageId) -> HashSet<PackageId> {
        let mut result = HashSet::new();
        let mut queue = vec![id.clone()];

        while let Some(current) = queue.pop() {
            for dep in self.get_dependencies(&current) {
                if result.insert(dep.clone()) {
                    queue.push(dep);
                }
            }
        }

        result
    }

    /// Find root packages (no dependents).
    pub fn find_roots(&self) -> Vec<PackageId> {
        self.nodes
            .read()
            .keys()
            .filter(|id| self.get_dependents(id).is_empty())
            .cloned()
            .collect()
    }

    /// Find leaf packages (no dependencies).
    pub fn find_leaves(&self) -> Vec<PackageId> {
        self.nodes
            .read()
            .keys()
            .filter(|id| self.get_dependencies(id).is_empty())
            .cloned()
            .collect()
    }

    /// Detect cycles in the graph.
    pub fn detect_cycles(&self) -> Vec<DependencyCycle> {
        let mut cycles = Vec::new();
        let nodes = self.nodes.read();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for id in nodes.keys() {
            if !visited.contains(id) {
                self.detect_cycles_dfs(id, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }

        cycles
    }

    fn detect_cycles_dfs(
        &self,
        id: &PackageId,
        visited: &mut HashSet<PackageId>,
        rec_stack: &mut HashSet<PackageId>,
        path: &mut Vec<PackageId>,
        cycles: &mut Vec<DependencyCycle>,
    ) {
        visited.insert(id.clone());
        rec_stack.insert(id.clone());
        path.push(id.clone());

        for dep in self.get_dependencies(id) {
            if !visited.contains(&dep) {
                self.detect_cycles_dfs(&dep, visited, rec_stack, path, cycles);
            } else if rec_stack.contains(&dep) {
                // Found a cycle.
                let cycle_start = path.iter().position(|p| p == &dep).unwrap();
                cycles.push(DependencyCycle {
                    packages: path[cycle_start..].to_vec(),
                });
            }
        }

        path.pop();
        rec_stack.remove(id);
    }

    /// Analyze the graph.
    pub fn analyze(&self) -> GraphAnalysis {
        let cycles = self.detect_cycles();
        let roots = self.find_roots();
        let leaves = self.find_leaves();

        GraphAnalysis {
            total_packages: self.nodes.read().len(),
            total_dependencies: self.edges.read().len(),
            root_packages: roots,
            leaf_packages: leaves,
            cycles,
            max_depth: self.calculate_max_depth(),
        }
    }

    fn calculate_max_depth(&self) -> usize {
        let roots = self.find_roots();
        let mut max_depth = 0;

        for root in roots {
            let depth = self.calculate_depth(&root, &mut HashSet::new());
            max_depth = max_depth.max(depth);
        }

        max_depth
    }

    fn calculate_depth(&self, id: &PackageId, visited: &mut HashSet<PackageId>) -> usize {
        if visited.contains(id) {
            return 0;
        }
        visited.insert(id.clone());

        let deps = self.get_dependencies(id);
        if deps.is_empty() {
            return 1;
        }

        1 + deps
            .iter()
            .map(|dep| self.calculate_depth(dep, visited))
            .max()
            .unwrap_or(0)
    }
}

impl Default for DependencyGraph {
    fn default() -> Self {
        Self::new()
    }
}
