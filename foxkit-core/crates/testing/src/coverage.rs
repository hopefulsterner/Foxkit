//! Code coverage

use std::path::PathBuf;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Coverage report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Coverage by file
    pub files: HashMap<PathBuf, FileCoverage>,
    /// Overall summary
    pub summary: CoverageSummary,
}

impl CoverageReport {
    /// Parse LCOV format
    pub fn from_lcov(content: &str) -> anyhow::Result<Self> {
        let mut report = Self::default();
        let mut current_file: Option<PathBuf> = None;
        let mut current_coverage: Option<FileCoverage> = None;

        for line in content.lines() {
            if line.starts_with("SF:") {
                // Source file
                let path = PathBuf::from(&line[3..]);
                current_file = Some(path.clone());
                current_coverage = Some(FileCoverage::new(path));
            } else if line.starts_with("DA:") {
                // Line coverage: DA:line,hits
                if let Some(ref mut cov) = current_coverage {
                    let parts: Vec<_> = line[3..].split(',').collect();
                    if parts.len() >= 2 {
                        if let (Ok(line), Ok(hits)) = (parts[0].parse(), parts[1].parse()) {
                            cov.lines.insert(line, hits);
                        }
                    }
                }
            } else if line.starts_with("BRDA:") {
                // Branch coverage: BRDA:line,block,branch,taken
                if let Some(ref mut cov) = current_coverage {
                    let parts: Vec<_> = line[5..].split(',').collect();
                    if parts.len() >= 4 {
                        if let (Ok(line), Ok(taken)) = (parts[0].parse(), parts[3].parse::<u32>()) {
                            cov.branches.push(BranchCoverage {
                                line,
                                taken: taken > 0,
                            });
                        }
                    }
                }
            } else if line.starts_with("FN:") {
                // Function: FN:line,name
                if let Some(ref mut cov) = current_coverage {
                    let parts: Vec<_> = line[3..].splitn(2, ',').collect();
                    if parts.len() >= 2 {
                        if let Ok(line) = parts[0].parse() {
                            cov.functions.push(FunctionCoverage {
                                name: parts[1].to_string(),
                                line,
                                hits: 0,
                            });
                        }
                    }
                }
            } else if line.starts_with("FNDA:") {
                // Function hits: FNDA:hits,name
                if let Some(ref mut cov) = current_coverage {
                    let parts: Vec<_> = line[5..].splitn(2, ',').collect();
                    if parts.len() >= 2 {
                        if let Ok(hits) = parts[0].parse() {
                            if let Some(func) = cov.functions.iter_mut()
                                .find(|f| f.name == parts[1])
                            {
                                func.hits = hits;
                            }
                        }
                    }
                }
            } else if line == "end_of_record" {
                // End of file record
                if let (Some(file), Some(cov)) = (current_file.take(), current_coverage.take()) {
                    report.files.insert(file, cov);
                }
            }
        }

        report.compute_summary();
        Ok(report)
    }

    /// Parse Cobertura XML format
    pub fn from_cobertura(content: &str) -> anyhow::Result<Self> {
        // Would parse XML
        Ok(Self::default())
    }

    /// Compute summary from file coverage
    fn compute_summary(&mut self) {
        let mut total_lines = 0;
        let mut covered_lines = 0;
        let mut total_branches = 0;
        let mut covered_branches = 0;
        let mut total_functions = 0;
        let mut covered_functions = 0;

        for cov in self.files.values() {
            total_lines += cov.lines.len();
            covered_lines += cov.lines.values().filter(|&&h| h > 0).count();
            
            total_branches += cov.branches.len();
            covered_branches += cov.branches.iter().filter(|b| b.taken).count();
            
            total_functions += cov.functions.len();
            covered_functions += cov.functions.iter().filter(|f| f.hits > 0).count();
        }

        self.summary = CoverageSummary {
            line_coverage: if total_lines > 0 {
                covered_lines as f64 / total_lines as f64
            } else {
                0.0
            },
            branch_coverage: if total_branches > 0 {
                covered_branches as f64 / total_branches as f64
            } else {
                0.0
            },
            function_coverage: if total_functions > 0 {
                covered_functions as f64 / total_functions as f64
            } else {
                0.0
            },
            total_lines,
            covered_lines,
            total_branches,
            covered_branches,
            total_functions,
            covered_functions,
        };
    }

    /// Get coverage for a file
    pub fn get_file(&self, path: &PathBuf) -> Option<&FileCoverage> {
        self.files.get(path)
    }

    /// Get line coverage for rendering
    pub fn get_line_decorations(&self, path: &PathBuf) -> Vec<LineCoverageDecoration> {
        self.files.get(path)
            .map(|cov| {
                cov.lines.iter()
                    .map(|(&line, &hits)| LineCoverageDecoration {
                        line,
                        covered: hits > 0,
                        hits,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// File coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    pub path: PathBuf,
    /// Line -> hit count
    pub lines: HashMap<u32, u32>,
    /// Branch coverage
    pub branches: Vec<BranchCoverage>,
    /// Function coverage
    pub functions: Vec<FunctionCoverage>,
}

impl FileCoverage {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            lines: HashMap::new(),
            branches: Vec::new(),
            functions: Vec::new(),
        }
    }

    /// Line coverage percentage
    pub fn line_coverage(&self) -> f64 {
        if self.lines.is_empty() {
            return 0.0;
        }
        let covered = self.lines.values().filter(|&&h| h > 0).count();
        covered as f64 / self.lines.len() as f64
    }
}

/// Branch coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchCoverage {
    pub line: u32,
    pub taken: bool,
}

/// Function coverage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCoverage {
    pub name: String,
    pub line: u32,
    pub hits: u32,
}

/// Coverage summary
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageSummary {
    pub line_coverage: f64,
    pub branch_coverage: f64,
    pub function_coverage: f64,
    pub total_lines: usize,
    pub covered_lines: usize,
    pub total_branches: usize,
    pub covered_branches: usize,
    pub total_functions: usize,
    pub covered_functions: usize,
}

impl CoverageSummary {
    /// Format as percentage string
    pub fn format_line_coverage(&self) -> String {
        format!("{:.1}%", self.line_coverage * 100.0)
    }

    /// Is coverage above threshold?
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.line_coverage >= threshold
    }
}

/// Line coverage decoration for editor
#[derive(Debug, Clone)]
pub struct LineCoverageDecoration {
    pub line: u32,
    pub covered: bool,
    pub hits: u32,
}
