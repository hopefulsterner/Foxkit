//! Code lens providers

use std::path::PathBuf;
use std::sync::Arc;
use async_trait::async_trait;

use crate::{CodeLens, CodeLensCommand, Range, TestLensData, TestKind};

/// Code lens provider trait
#[async_trait]
pub trait CodeLensProvider: Send + Sync {
    /// Provider ID
    fn id(&self) -> &str;

    /// Provide code lenses for file
    async fn provide_lenses(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<CodeLens>>;
}

/// Built-in providers
pub struct BuiltinProviders;

impl BuiltinProviders {
    pub fn all() -> Vec<Arc<dyn CodeLensProvider>> {
        vec![
            Arc::new(ReferenceProvider),
            Arc::new(TestProvider),
            Arc::new(ImplementationProvider),
            Arc::new(RunProvider),
        ]
    }
}

/// Reference count provider
pub struct ReferenceProvider;

#[async_trait]
impl CodeLensProvider for ReferenceProvider {
    fn id(&self) -> &str {
        "references"
    }

    async fn provide_lenses(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<CodeLens>> {
        let mut lenses = Vec::new();

        // Find function/struct/trait definitions and add reference counts
        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Check for function definitions
            if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                lenses.push(create_reference_lens(line_num as u32, file));
            }
            
            // Check for struct definitions
            if trimmed.starts_with("pub struct ") || trimmed.starts_with("struct ") {
                lenses.push(create_reference_lens(line_num as u32, file));
            }
            
            // Check for trait definitions
            if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                lenses.push(create_reference_lens(line_num as u32, file));
            }
            
            // Check for impl blocks
            if trimmed.starts_with("impl ") || trimmed.starts_with("impl<") {
                lenses.push(create_reference_lens(line_num as u32, file));
            }
        }

        Ok(lenses)
    }
}

fn create_reference_lens(line: u32, _file: &PathBuf) -> CodeLens {
    CodeLens::new(Range::line(line))
        .with_data(serde_json::json!({
            "type": "references",
            "line": line
        }))
}

/// Test provider
pub struct TestProvider;

#[async_trait]
impl CodeLensProvider for TestProvider {
    fn id(&self) -> &str {
        "tests"
    }

    async fn provide_lenses(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<CodeLens>> {
        let mut lenses = Vec::new();

        let is_test_file = file.to_string_lossy().contains("test")
            || file.to_string_lossy().contains("spec");

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Rust tests
            if trimmed.starts_with("#[test]") {
                if let Some(next_fn) = find_next_function(content, line_num) {
                    lenses.push(create_test_lens(line_num as u32, &next_fn, file, TestKind::Unit));
                }
            }
            
            // Rust benchmarks
            if trimmed.starts_with("#[bench]") {
                if let Some(next_fn) = find_next_function(content, line_num) {
                    lenses.push(create_test_lens(line_num as u32, &next_fn, file, TestKind::Benchmark));
                }
            }
            
            // JavaScript/TypeScript tests
            if (trimmed.starts_with("it(") || trimmed.starts_with("test(") || trimmed.starts_with("describe("))
                && is_test_file
            {
                let test_name = extract_test_name(trimmed);
                lenses.push(create_test_lens(line_num as u32, &test_name, file, TestKind::Unit));
            }
            
            // Python tests
            if trimmed.starts_with("def test_") {
                let test_name = extract_python_function_name(trimmed);
                lenses.push(create_test_lens(line_num as u32, &test_name, file, TestKind::Unit));
            }
        }

        Ok(lenses)
    }
}

fn find_next_function(content: &str, from_line: usize) -> Option<String> {
    for line in content.lines().skip(from_line + 1) {
        let trimmed = line.trim();
        if trimmed.starts_with("fn ") || trimmed.starts_with("async fn ") {
            // Extract function name
            let start = trimmed.find("fn ")? + 3;
            let end = trimmed[start..].find('(')?;
            return Some(trimmed[start..start + end].to_string());
        }
    }
    None
}

fn extract_test_name(line: &str) -> String {
    // Extract string from it("name", ...) or test("name", ...)
    if let Some(start) = line.find('"') {
        if let Some(end) = line[start + 1..].find('"') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    if let Some(start) = line.find('\'') {
        if let Some(end) = line[start + 1..].find('\'') {
            return line[start + 1..start + 1 + end].to_string();
        }
    }
    "test".to_string()
}

fn extract_python_function_name(line: &str) -> String {
    if let Some(start) = line.find("def ") {
        let rest = &line[start + 4..];
        if let Some(end) = rest.find('(') {
            return rest[..end].to_string();
        }
    }
    "test".to_string()
}

fn create_test_lens(line: u32, test_name: &str, file: &PathBuf, kind: TestKind) -> CodeLens {
    let data = TestLensData {
        test_name: test_name.to_string(),
        test_file: file.clone(),
        kind,
    };
    
    CodeLens::new(Range::line(line))
        .with_command(CodeLensCommand {
            title: "▶ Run Test".to_string(),
            command: "foxkit.runTest".to_string(),
            arguments: vec![serde_json::to_value(&data).unwrap()],
        })
}

/// Implementation provider
pub struct ImplementationProvider;

#[async_trait]
impl CodeLensProvider for ImplementationProvider {
    fn id(&self) -> &str {
        "implementations"
    }

    async fn provide_lenses(&self, _file: &PathBuf, content: &str) -> anyhow::Result<Vec<CodeLens>> {
        let mut lenses = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Traits
            if trimmed.starts_with("pub trait ") || trimmed.starts_with("trait ") {
                lenses.push(CodeLens::new(Range::line(line_num as u32))
                    .with_data(serde_json::json!({
                        "type": "implementations",
                        "line": line_num
                    })));
            }
            
            // Interfaces (TypeScript)
            if trimmed.starts_with("export interface ") || trimmed.starts_with("interface ") {
                lenses.push(CodeLens::new(Range::line(line_num as u32))
                    .with_data(serde_json::json!({
                        "type": "implementations",
                        "line": line_num
                    })));
            }
            
            // Abstract classes
            if trimmed.starts_with("abstract class ") {
                lenses.push(CodeLens::new(Range::line(line_num as u32))
                    .with_data(serde_json::json!({
                        "type": "implementations",
                        "line": line_num
                    })));
            }
        }

        Ok(lenses)
    }
}

/// Run provider (for main/runnable functions)
pub struct RunProvider;

#[async_trait]
impl CodeLensProvider for RunProvider {
    fn id(&self) -> &str {
        "run"
    }

    async fn provide_lenses(&self, file: &PathBuf, content: &str) -> anyhow::Result<Vec<CodeLens>> {
        let mut lenses = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            
            // Rust main function
            if trimmed == "fn main() {" || trimmed.starts_with("fn main()") {
                lenses.push(create_run_lens(line_num as u32, file, "main"));
            }
            
            // JavaScript/TypeScript entry point detection
            if file.extension().map(|e| e == "js" || e == "ts" || e == "mjs").unwrap_or(false) {
                // Check for main execution patterns
                if trimmed.starts_with("// @main") || trimmed.starts_with("/* @main") {
                    lenses.push(create_run_lens(line_num as u32, file, "script"));
                }
            }
            
            // Python main block
            if trimmed == "if __name__ == \"__main__\":" || trimmed == "if __name__ == '__main__':" {
                lenses.push(create_run_lens(line_num as u32, file, "main"));
            }
        }

        Ok(lenses)
    }
}

fn create_run_lens(line: u32, file: &PathBuf, entry: &str) -> CodeLens {
    CodeLens::new(Range::line(line))
        .with_command(CodeLensCommand {
            title: "▶ Run".to_string(),
            command: "foxkit.run".to_string(),
            arguments: vec![serde_json::json!({
                "file": file,
                "entry": entry
            })],
        })
}
