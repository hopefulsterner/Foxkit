//! Test discovery

use std::path::PathBuf;
use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use regex::Regex;

use crate::TestId;

/// Test discovery service
pub struct TestDiscovery {
    /// Discovered tests by file
    tests: RwLock<HashMap<PathBuf, Vec<TestItem>>>,
    /// Discovery patterns by language
    patterns: HashMap<String, Vec<TestPattern>>,
}

impl TestDiscovery {
    pub fn new() -> Self {
        let mut patterns = HashMap::new();
        
        // Rust patterns
        patterns.insert("rust".to_string(), vec![
            TestPattern {
                pattern: Regex::new(r"#\[test\]").unwrap(),
                kind: TestPatternKind::Attribute,
            },
            TestPattern {
                pattern: Regex::new(r"#\[tokio::test\]").unwrap(),
                kind: TestPatternKind::Attribute,
            },
            TestPattern {
                pattern: Regex::new(r"#\[bench\]").unwrap(),
                kind: TestPatternKind::Benchmark,
            },
        ]);
        
        // JavaScript/TypeScript patterns
        patterns.insert("javascript".to_string(), vec![
            TestPattern {
                pattern: Regex::new(r#"(it|test)\s*\(\s*['"`]"#).unwrap(),
                kind: TestPatternKind::FunctionCall,
            },
            TestPattern {
                pattern: Regex::new(r#"describe\s*\(\s*['"`]"#).unwrap(),
                kind: TestPatternKind::Suite,
            },
        ]);
        
        // Python patterns
        patterns.insert("python".to_string(), vec![
            TestPattern {
                pattern: Regex::new(r"def\s+test_").unwrap(),
                kind: TestPatternKind::Function,
            },
            TestPattern {
                pattern: Regex::new(r"class\s+Test").unwrap(),
                kind: TestPatternKind::Class,
            },
        ]);

        Self {
            tests: RwLock::new(HashMap::new()),
            patterns,
        }
    }

    /// Discover tests in workspace
    pub async fn discover(&self, workspace: &PathBuf) -> anyhow::Result<Vec<TestItem>> {
        let mut all_tests = Vec::new();

        // Walk workspace and find test files
        for entry in walkdir(workspace)? {
            if let Some(tests) = self.discover_file(&entry).await? {
                all_tests.extend(tests.clone());
                self.tests.write().insert(entry, tests);
            }
        }

        Ok(all_tests)
    }

    /// Discover tests in a single file
    pub async fn discover_file(&self, file: &PathBuf) -> anyhow::Result<Option<Vec<TestItem>>> {
        let lang = detect_language(file);
        
        let patterns = match lang {
            Some(l) => self.patterns.get(l),
            None => return Ok(None),
        };

        let patterns = match patterns {
            Some(p) => p,
            None => return Ok(None),
        };

        let content = tokio::fs::read_to_string(file).await?;
        let mut tests = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            for pattern in patterns {
                if pattern.pattern.is_match(line) {
                    // Look for the actual test function/method
                    if let Some(item) = self.extract_test_item(
                        file,
                        &content,
                        line_num,
                        &pattern.kind,
                    ) {
                        tests.push(item);
                    }
                }
            }
        }

        if tests.is_empty() {
            Ok(None)
        } else {
            Ok(Some(tests))
        }
    }

    /// Extract test item from source
    fn extract_test_item(
        &self,
        file: &PathBuf,
        content: &str,
        line_num: usize,
        kind: &TestPatternKind,
    ) -> Option<TestItem> {
        let lines: Vec<_> = content.lines().collect();
        
        match kind {
            TestPatternKind::Attribute => {
                // Look for function on next line
                for i in (line_num + 1)..lines.len().min(line_num + 5) {
                    let line = lines[i].trim();
                    if line.starts_with("fn ") || line.starts_with("async fn ") {
                        let name = extract_function_name(line)?;
                        return Some(TestItem {
                            id: TestId::new(format!("{}::{}", file.display(), name)),
                            name,
                            kind: TestItemKind::Test,
                            file: file.clone(),
                            line: i as u32,
                            children: Vec::new(),
                        });
                    }
                }
            }
            TestPatternKind::FunctionCall => {
                // Extract test name from it("name", ...) or test("name", ...)
                let line = lines[line_num];
                let name = extract_string_arg(line)?;
                return Some(TestItem {
                    id: TestId::new(format!("{}::{}", file.display(), name)),
                    name,
                    kind: TestItemKind::Test,
                    file: file.clone(),
                    line: line_num as u32,
                    children: Vec::new(),
                });
            }
            TestPatternKind::Suite => {
                // describe block - extract name and find child tests
                let line = lines[line_num];
                let name = extract_string_arg(line)?;
                return Some(TestItem {
                    id: TestId::new(format!("{}::{}", file.display(), name)),
                    name,
                    kind: TestItemKind::Suite,
                    file: file.clone(),
                    line: line_num as u32,
                    children: Vec::new(), // Would recursively find
                });
            }
            TestPatternKind::Function => {
                let line = lines[line_num].trim();
                let name = extract_python_function_name(line)?;
                return Some(TestItem {
                    id: TestId::new(format!("{}::{}", file.display(), name)),
                    name,
                    kind: TestItemKind::Test,
                    file: file.clone(),
                    line: line_num as u32,
                    children: Vec::new(),
                });
            }
            TestPatternKind::Class => {
                let line = lines[line_num].trim();
                let name = extract_python_class_name(line)?;
                return Some(TestItem {
                    id: TestId::new(format!("{}::{}", file.display(), name)),
                    name,
                    kind: TestItemKind::Suite,
                    file: file.clone(),
                    line: line_num as u32,
                    children: Vec::new(),
                });
            }
            TestPatternKind::Benchmark => {
                // Similar to Attribute but for benchmarks
                for i in (line_num + 1)..lines.len().min(line_num + 5) {
                    let line = lines[i].trim();
                    if line.starts_with("fn ") {
                        let name = extract_function_name(line)?;
                        return Some(TestItem {
                            id: TestId::new(format!("{}::{}", file.display(), name)),
                            name,
                            kind: TestItemKind::Benchmark,
                            file: file.clone(),
                            line: i as u32,
                            children: Vec::new(),
                        });
                    }
                }
            }
        }

        None
    }

    /// Get cached tests for file
    pub fn get_tests(&self, file: &PathBuf) -> Option<Vec<TestItem>> {
        self.tests.read().get(file).cloned()
    }

    /// Clear cache
    pub fn clear(&self) {
        self.tests.write().clear();
    }
}

impl Default for TestDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

/// Test item
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestItem {
    pub id: TestId,
    pub name: String,
    pub kind: TestItemKind,
    pub file: PathBuf,
    pub line: u32,
    pub children: Vec<TestItem>,
}

/// Test item kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TestItemKind {
    Test,
    Suite,
    Benchmark,
    DocTest,
}

/// Test pattern
struct TestPattern {
    pattern: Regex,
    kind: TestPatternKind,
}

/// Test pattern kind
#[derive(Debug, Clone)]
enum TestPatternKind {
    Attribute,
    FunctionCall,
    Suite,
    Function,
    Class,
    Benchmark,
}

fn detect_language(file: &PathBuf) -> Option<&'static str> {
    let ext = file.extension()?.to_str()?;
    match ext {
        "rs" => Some("rust"),
        "ts" | "tsx" | "js" | "jsx" | "mjs" => Some("javascript"),
        "py" => Some("python"),
        _ => None,
    }
}

fn walkdir(path: &PathBuf) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    
    fn walk(dir: &PathBuf, files: &mut Vec<PathBuf>) -> anyhow::Result<()> {
        if dir.is_file() {
            files.push(dir.clone());
            return Ok(());
        }
        
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            
            // Skip hidden and common non-source directories
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.starts_with('.') || name == "node_modules" || name == "target" {
                continue;
            }
            
            if path.is_dir() {
                walk(&path, files)?;
            } else if path.is_file() {
                files.push(path);
            }
        }
        
        Ok(())
    }
    
    walk(path, &mut files)?;
    Ok(files)
}

fn extract_function_name(line: &str) -> Option<String> {
    let line = line.trim();
    let start = if line.starts_with("async fn ") {
        9
    } else if line.starts_with("fn ") {
        3
    } else {
        return None;
    };
    
    let rest = &line[start..];
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    Some(rest[..end].to_string())
}

fn extract_string_arg(line: &str) -> Option<String> {
    // Find first string in line
    for quote in ['"', '\'', '`'] {
        if let Some(start) = line.find(quote) {
            if let Some(end) = line[start + 1..].find(quote) {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }
    }
    None
}

fn extract_python_function_name(line: &str) -> Option<String> {
    let start = line.find("def ")? + 4;
    let rest = &line[start..];
    let end = rest.find('(')?;
    Some(rest[..end].to_string())
}

fn extract_python_class_name(line: &str) -> Option<String> {
    let start = line.find("class ")? + 6;
    let rest = &line[start..];
    let end = rest.find(|c: char| !c.is_alphanumeric() && c != '_')?;
    Some(rest[..end].to_string())
}
