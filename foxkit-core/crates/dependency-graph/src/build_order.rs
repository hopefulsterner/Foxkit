//! Build Order Computation
//!
//! Computes optimal build order based on dependency graph.

use std::collections::{HashMap, HashSet, VecDeque};

/// Build node representing a package to build
#[derive(Debug, Clone)]
pub struct BuildNode {
    pub name: String,
    pub path: String,
    pub dependencies: Vec<String>,
}

/// Build plan with ordered stages
#[derive(Debug, Clone)]
pub struct BuildPlan {
    /// Stages in order (each stage can be built in parallel)
    pub stages: Vec<BuildStage>,
    /// Total number of packages
    pub total_packages: usize,
    /// Detected cycles (if any)
    pub cycles: Vec<Vec<String>>,
}

/// A build stage containing packages that can be built in parallel
#[derive(Debug, Clone)]
pub struct BuildStage {
    pub index: usize,
    pub packages: Vec<String>,
}

/// Build order computer
pub struct BuildOrderComputer;

impl BuildOrderComputer {
    /// Compute build order using topological sort
    pub fn compute(packages: &[BuildNode]) -> BuildPlan {
        let mut graph: HashMap<String, Vec<String>> = HashMap::new();
        let mut in_degree: HashMap<String, usize> = HashMap::new();
        
        // Initialize graph
        for pkg in packages {
            graph.entry(pkg.name.clone()).or_default();
            in_degree.entry(pkg.name.clone()).or_insert(0);
            
            for dep in &pkg.dependencies {
                graph.entry(dep.clone()).or_default().push(pkg.name.clone());
                *in_degree.entry(pkg.name.clone()).or_insert(0) += 1;
            }
        }
        
        // Find cycles first
        let cycles = Self::detect_cycles(&graph, packages);
        
        // Kahn's algorithm for topological sort with stages
        let mut stages = Vec::new();
        let mut queue: VecDeque<String> = in_degree.iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(name, _)| name.clone())
            .collect();
        
        let mut remaining = in_degree.clone();
        let mut stage_index = 0;
        
        while !queue.is_empty() {
            let mut current_stage = Vec::new();
            let stage_size = queue.len();
            
            for _ in 0..stage_size {
                if let Some(node) = queue.pop_front() {
                    current_stage.push(node.clone());
                    
                    if let Some(dependents) = graph.get(&node) {
                        for dependent in dependents {
                            if let Some(deg) = remaining.get_mut(dependent) {
                                *deg -= 1;
                                if *deg == 0 {
                                    queue.push_back(dependent.clone());
                                }
                            }
                        }
                    }
                }
            }
            
            if !current_stage.is_empty() {
                stages.push(BuildStage {
                    index: stage_index,
                    packages: current_stage,
                });
                stage_index += 1;
            }
        }
        
        BuildPlan {
            stages,
            total_packages: packages.len(),
            cycles,
        }
    }

    fn detect_cycles(graph: &HashMap<String, Vec<String>>, packages: &[BuildNode]) -> Vec<Vec<String>> {
        let mut cycles = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();
        
        // Build reverse graph (dependency -> dependents)
        let mut dep_graph: HashMap<String, Vec<String>> = HashMap::new();
        for pkg in packages {
            for dep in &pkg.dependencies {
                dep_graph.entry(pkg.name.clone()).or_default().push(dep.clone());
            }
        }
        
        for pkg in packages {
            if !visited.contains(&pkg.name) {
                Self::dfs_cycles(&dep_graph, &pkg.name, &mut visited, &mut rec_stack, &mut path, &mut cycles);
            }
        }
        
        cycles
    }

    fn dfs_cycles(
        graph: &HashMap<String, Vec<String>>,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
        cycles: &mut Vec<Vec<String>>,
    ) {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());
        
        if let Some(neighbors) = graph.get(node) {
            for neighbor in neighbors {
                if !visited.contains(neighbor) {
                    Self::dfs_cycles(graph, neighbor, visited, rec_stack, path, cycles);
                } else if rec_stack.contains(neighbor) {
                    // Found cycle
                    let cycle_start = path.iter().position(|n| n == neighbor).unwrap();
                    let cycle: Vec<String> = path[cycle_start..].to_vec();
                    cycles.push(cycle);
                }
            }
        }
        
        path.pop();
        rec_stack.remove(node);
    }

    /// Get affected packages when a package changes
    pub fn affected_packages(packages: &[BuildNode], changed: &[String]) -> Vec<String> {
        // Build reverse dependency graph
        let mut reverse_deps: HashMap<String, Vec<String>> = HashMap::new();
        for pkg in packages {
            for dep in &pkg.dependencies {
                reverse_deps.entry(dep.clone()).or_default().push(pkg.name.clone());
            }
        }
        
        // BFS to find all affected packages
        let mut affected = HashSet::new();
        let mut queue: VecDeque<String> = changed.iter().cloned().collect();
        
        while let Some(pkg) = queue.pop_front() {
            if affected.insert(pkg.clone()) {
                if let Some(dependents) = reverse_deps.get(&pkg) {
                    for dependent in dependents {
                        if !affected.contains(dependent) {
                            queue.push_back(dependent.clone());
                        }
                    }
                }
            }
        }
        
        affected.into_iter().collect()
    }

    /// Get direct dependents of a package
    pub fn direct_dependents(packages: &[BuildNode], package: &str) -> Vec<String> {
        packages.iter()
            .filter(|p| p.dependencies.contains(&package.to_string()))
            .map(|p| p.name.clone())
            .collect()
    }

    /// Get direct dependencies of a package
    pub fn direct_dependencies(packages: &[BuildNode], package: &str) -> Vec<String> {
        packages.iter()
            .find(|p| p.name == package)
            .map(|p| p.dependencies.clone())
            .unwrap_or_default()
    }

    /// Get transitive dependencies
    pub fn transitive_dependencies(packages: &[BuildNode], package: &str) -> Vec<String> {
        let pkg_map: HashMap<String, &BuildNode> = packages.iter()
            .map(|p| (p.name.clone(), p))
            .collect();
        
        let mut deps = HashSet::new();
        let mut queue: VecDeque<String> = pkg_map.get(package)
            .map(|p| p.dependencies.clone())
            .unwrap_or_default()
            .into_iter()
            .collect();
        
        while let Some(dep) = queue.pop_front() {
            if deps.insert(dep.clone()) {
                if let Some(pkg) = pkg_map.get(&dep) {
                    for subdep in &pkg.dependencies {
                        if !deps.contains(subdep) {
                            queue.push_back(subdep.clone());
                        }
                    }
                }
            }
        }
        
        deps.into_iter().collect()
    }
}

/// Incremental build support
pub struct IncrementalBuild {
    /// Previously built packages with their hashes
    built: HashMap<String, String>,
}

impl IncrementalBuild {
    pub fn new() -> Self {
        Self { built: HashMap::new() }
    }

    /// Record a successful build
    pub fn record_build(&mut self, package: &str, hash: &str) {
        self.built.insert(package.to_string(), hash.to_string());
    }

    /// Check if a package needs rebuilding
    pub fn needs_rebuild(&self, package: &str, current_hash: &str) -> bool {
        match self.built.get(package) {
            Some(prev_hash) => prev_hash != current_hash,
            None => true,
        }
    }

    /// Filter build plan to only include packages that need rebuilding
    pub fn filter_plan(&self, plan: &BuildPlan, hashes: &HashMap<String, String>) -> BuildPlan {
        let needs_rebuild: HashSet<String> = hashes.iter()
            .filter(|(pkg, hash)| self.needs_rebuild(pkg, hash))
            .map(|(pkg, _)| pkg.clone())
            .collect();
        
        let filtered_stages: Vec<BuildStage> = plan.stages.iter()
            .map(|stage| BuildStage {
                index: stage.index,
                packages: stage.packages.iter()
                    .filter(|p| needs_rebuild.contains(*p))
                    .cloned()
                    .collect(),
            })
            .filter(|stage| !stage.packages.is_empty())
            .enumerate()
            .map(|(i, mut s)| { s.index = i; s })
            .collect();
        
        BuildPlan {
            stages: filtered_stages,
            total_packages: needs_rebuild.len(),
            cycles: plan.cycles.clone(),
        }
    }

    /// Clear build cache
    pub fn clear(&mut self) {
        self.built.clear();
    }
}

impl Default for IncrementalBuild {
    fn default() -> Self {
        Self::new()
    }
}
