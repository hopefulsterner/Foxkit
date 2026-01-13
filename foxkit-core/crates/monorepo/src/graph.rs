//! Dependency graph construction and analysis

use std::collections::{HashMap, HashSet};
use anyhow::Result;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use petgraph::Direction;

use crate::package::Package;

/// Dependency graph of the monorepo
pub struct DependencyGraph {
    /// The underlying directed graph
    graph: DiGraph<String, ()>,
    /// Map from package name to node index
    node_map: HashMap<String, NodeIndex>,
}

impl DependencyGraph {
    /// Build a dependency graph from packages
    pub fn build(packages: &[Package]) -> Result<Self> {
        let mut graph = DiGraph::new();
        let mut node_map = HashMap::new();
        
        // First pass: add all nodes
        for pkg in packages {
            let idx = graph.add_node(pkg.name.clone());
            node_map.insert(pkg.name.clone(), idx);
        }
        
        // Second pass: add edges for internal dependencies
        let internal_names: HashSet<_> = packages.iter().map(|p| &p.name).collect();
        
        for pkg in packages {
            let from_idx = node_map[&pkg.name];
            
            for dep in pkg.all_dependencies() {
                // Only track internal dependencies
                if internal_names.contains(&dep.to_string()) {
                    let to_idx = node_map[dep];
                    graph.add_edge(from_idx, to_idx, ());
                }
            }
        }
        
        Ok(Self { graph, node_map })
    }

    /// Get topologically sorted build order
    pub fn topological_order(&self) -> Result<Vec<String>> {
        let sorted = toposort(&self.graph, None)
            .map_err(|_| anyhow::anyhow!("Circular dependency detected"))?;
        
        // Reverse because toposort gives us dependencies first
        Ok(sorted
            .into_iter()
            .rev()
            .map(|idx| self.graph[idx].clone())
            .collect())
    }

    /// Get all packages that depend on the given packages (directly or transitively)
    pub fn dependents(&self, changed: &[String]) -> Result<Vec<String>> {
        let mut affected = HashSet::new();
        let mut queue: Vec<_> = changed.iter().cloned().collect();
        
        while let Some(pkg) = queue.pop() {
            if let Some(&idx) = self.node_map.get(&pkg) {
                // Find all packages that depend on this one
                for neighbor in self.graph.neighbors_directed(idx, Direction::Incoming) {
                    let name = &self.graph[neighbor];
                    if affected.insert(name.clone()) {
                        queue.push(name.clone());
                    }
                }
            }
        }
        
        Ok(affected.into_iter().collect())
    }

    /// Get direct dependencies of a package
    pub fn dependencies_of(&self, name: &str) -> Vec<String> {
        self.node_map.get(name)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Outgoing)
                    .map(|n| self.graph[n].clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Get direct dependents of a package
    pub fn dependents_of(&self, name: &str) -> Vec<String> {
        self.node_map.get(name)
            .map(|&idx| {
                self.graph
                    .neighbors_directed(idx, Direction::Incoming)
                    .map(|n| self.graph[n].clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check for circular dependencies
    pub fn has_cycles(&self) -> bool {
        toposort(&self.graph, None).is_err()
    }

    /// Get all package names
    pub fn packages(&self) -> Vec<&str> {
        self.node_map.keys().map(|s| s.as_str()).collect()
    }

    /// Export to DOT format for visualization
    pub fn to_dot(&self) -> String {
        use petgraph::dot::{Dot, Config};
        format!("{:?}", Dot::with_config(&self.graph, &[Config::EdgeNoLabel]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::package::{PackageKind, PackageManager};

    fn make_package(name: &str, deps: Vec<&str>) -> Package {
        Package {
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            path: std::path::PathBuf::from(name),
            kind: PackageKind::Library,
            package_manager: Some(PackageManager::Npm),
            build_system: None,
            dependencies: deps.into_iter().map(String::from).collect(),
            dev_dependencies: vec![],
            peer_dependencies: vec![],
            source_files: vec![],
            entry_points: vec![],
        }
    }

    #[test]
    fn test_topological_order() {
        let packages = vec![
            make_package("app", vec!["ui", "utils"]),
            make_package("ui", vec!["utils"]),
            make_package("utils", vec![]),
        ];
        
        let graph = DependencyGraph::build(&packages).unwrap();
        let order = graph.topological_order().unwrap();
        
        // utils should come before ui, ui before app
        let utils_pos = order.iter().position(|n| n == "utils").unwrap();
        let ui_pos = order.iter().position(|n| n == "ui").unwrap();
        let app_pos = order.iter().position(|n| n == "app").unwrap();
        
        assert!(utils_pos < ui_pos);
        assert!(ui_pos < app_pos);
    }

    #[test]
    fn test_dependents() {
        let packages = vec![
            make_package("app", vec!["ui", "utils"]),
            make_package("ui", vec!["utils"]),
            make_package("utils", vec![]),
        ];
        
        let graph = DependencyGraph::build(&packages).unwrap();
        let affected = graph.dependents(&["utils".to_string()]).unwrap();
        
        // Both app and ui depend on utils
        assert!(affected.contains(&"app".to_string()));
        assert!(affected.contains(&"ui".to_string()));
    }
}
