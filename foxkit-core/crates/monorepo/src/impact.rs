//! Impact analysis - what breaks if I change this?

use std::sync::Arc;
use anyhow::Result;

use crate::package::Package;
use crate::graph::DependencyGraph;

/// Impact analysis result
#[derive(Debug, Clone)]
pub struct ImpactAnalysis {
    /// The package being analyzed
    pub package: String,
    /// Direct dependents (packages that import this one)
    pub direct_dependents: Vec<String>,
    /// Transitive dependents (all affected packages)
    pub transitive_dependents: Vec<String>,
    /// Estimated impact level
    pub impact_level: ImpactLevel,
    /// Suggested actions
    pub suggestions: Vec<String>,
}

/// Impact severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImpactLevel {
    /// No other packages affected
    None,
    /// Only a few packages affected
    Low,
    /// Moderate number of packages affected
    Medium,
    /// Many packages affected
    High,
    /// Critical - affects most of the monorepo
    Critical,
}

/// Analyze the impact of changes to a package
pub fn analyze(package: &Arc<Package>, graph: &Arc<DependencyGraph>) -> Result<ImpactAnalysis> {
    let direct = graph.dependents_of(&package.name);
    let transitive = graph.dependents(&[package.name.clone()])?;
    
    let total_packages = graph.packages().len();
    let affected_ratio = transitive.len() as f32 / total_packages as f32;
    
    let impact_level = if transitive.is_empty() {
        ImpactLevel::None
    } else if affected_ratio < 0.1 {
        ImpactLevel::Low
    } else if affected_ratio < 0.3 {
        ImpactLevel::Medium
    } else if affected_ratio < 0.6 {
        ImpactLevel::High
    } else {
        ImpactLevel::Critical
    };
    
    let mut suggestions = Vec::new();
    
    match impact_level {
        ImpactLevel::Critical | ImpactLevel::High => {
            suggestions.push("Consider breaking change review process".to_string());
            suggestions.push("Run full test suite before merging".to_string());
            suggestions.push("Consider feature flag for gradual rollout".to_string());
        }
        ImpactLevel::Medium => {
            suggestions.push("Run tests for affected packages".to_string());
            suggestions.push("Review API changes carefully".to_string());
        }
        ImpactLevel::Low => {
            suggestions.push("Run affected package tests".to_string());
        }
        ImpactLevel::None => {
            suggestions.push("Safe to change - no dependents".to_string());
        }
    }
    
    Ok(ImpactAnalysis {
        package: package.name.clone(),
        direct_dependents: direct,
        transitive_dependents: transitive,
        impact_level,
        suggestions,
    })
}

impl std::fmt::Display for ImpactLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImpactLevel::None => write!(f, "None"),
            ImpactLevel::Low => write!(f, "Low"),
            ImpactLevel::Medium => write!(f, "Medium"),
            ImpactLevel::High => write!(f, "High"),
            ImpactLevel::Critical => write!(f, "Critical"),
        }
    }
}
