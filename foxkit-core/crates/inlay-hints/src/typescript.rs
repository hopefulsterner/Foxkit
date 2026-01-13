//! TypeScript inlay hint provider

use std::path::PathBuf;
use async_trait::async_trait;

use crate::{InlayHint, InlayHintsConfig, InlayHintProvider, Position, ParameterHintsMode, provider::detect_language};

/// TypeScript inlay hint provider
pub struct TypeScriptInlayHintProvider;

impl TypeScriptInlayHintProvider {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TypeScriptInlayHintProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl InlayHintProvider for TypeScriptInlayHintProvider {
    fn id(&self) -> &str {
        "typescript"
    }

    fn languages(&self) -> &[&str] {
        &["typescript", "javascript", "typescriptreact", "javascriptreact"]
    }

    async fn provide_hints(
        &self,
        file: &PathBuf,
        content: &str,
        config: &InlayHintsConfig,
    ) -> anyhow::Result<Vec<InlayHint>> {
        let lang = detect_language(file);
        
        // Check if this is a TypeScript/JavaScript file
        if lang != Some("typescript") && lang != Some("javascript") {
            return Ok(Vec::new());
        }

        let mut hints = Vec::new();

        for (line_num, line) in content.lines().enumerate() {
            // Type hints for variable declarations
            if config.show_type_hints {
                hints.extend(find_variable_type_hints(line, line_num as u32));
            }

            // Parameter hints for function calls
            if config.show_parameter_hints {
                hints.extend(find_parameter_hints(line, line_num as u32, &config.parameter_hints_mode));
            }

            // Return type hints for functions without explicit return type
            hints.extend(find_return_type_hints(line, line_num as u32));
        }

        Ok(hints)
    }
}

/// Find type hints for variable declarations
fn find_variable_type_hints(line: &str, line_num: u32) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let trimmed = line.trim();

    // const name = ...
    // let name = ...
    // var name = ...
    for keyword in ["const ", "let ", "var "] {
        if trimmed.starts_with(keyword) {
            let rest = &trimmed[keyword.len()..];
            
            // Find the variable name
            if let Some(eq_pos) = rest.find('=') {
                let before_eq = &rest[..eq_pos].trim();
                
                // Skip if already has type annotation
                if !before_eq.contains(':') {
                    // Destructuring - skip for now (complex)
                    if before_eq.starts_with('{') || before_eq.starts_with('[') {
                        continue;
                    }
                    
                    let col = line.find(before_eq).unwrap_or(0) + before_eq.len();
                    hints.push(InlayHint::type_hint(
                        Position::new(line_num, col as u32),
                        "/* inferred */".to_string(),
                    ));
                }
            }
        }
    }

    hints
}

/// Find parameter hints for function calls
fn find_parameter_hints(line: &str, line_num: u32, mode: &ParameterHintsMode) -> Vec<InlayHint> {
    let mut hints = Vec::new();

    if matches!(mode, ParameterHintsMode::None) {
        return hints;
    }

    // Simple heuristic: find function calls with literal arguments
    // foo(true, 42, "hello")
    
    let mut in_call = false;
    let mut paren_depth = 0;
    let mut arg_start = 0;
    let mut col = 0;

    for (i, c) in line.char_indices() {
        match c {
            '(' => {
                if paren_depth == 0 {
                    in_call = true;
                    arg_start = i + 1;
                }
                paren_depth += 1;
            }
            ')' => {
                paren_depth -= 1;
                if paren_depth == 0 {
                    in_call = false;
                }
            }
            ',' if in_call && paren_depth == 1 => {
                // Check if argument is a literal
                let arg = line[arg_start..i].trim();
                if matches!(mode, ParameterHintsMode::All) || is_literal(arg) {
                    // Would need function signature to know parameter name
                    // Placeholder
                }
                arg_start = i + 1;
            }
            _ => {}
        }
        col = i;
    }

    hints
}

/// Check if a string looks like a literal
fn is_literal(s: &str) -> bool {
    let s = s.trim();
    
    // Number literal
    if s.parse::<f64>().is_ok() {
        return true;
    }
    
    // Boolean literal
    if s == "true" || s == "false" {
        return true;
    }
    
    // String literal
    if (s.starts_with('"') && s.ends_with('"')) ||
       (s.starts_with('\'') && s.ends_with('\'')) ||
       (s.starts_with('`') && s.ends_with('`')) {
        return true;
    }
    
    // Null/undefined
    if s == "null" || s == "undefined" {
        return true;
    }
    
    // Object/array literal
    if (s.starts_with('{') && s.ends_with('}')) ||
       (s.starts_with('[') && s.ends_with(']')) {
        return true;
    }
    
    false
}

/// Find return type hints for functions
fn find_return_type_hints(line: &str, line_num: u32) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let trimmed = line.trim();

    // function name() { ... }
    // const name = () => { ... }
    // const name = function() { ... }
    
    // Arrow function without return type
    if trimmed.contains("=>") {
        if let Some(arrow_pos) = trimmed.find("=>") {
            // Check if there's a return type before =>
            let before_arrow = &trimmed[..arrow_pos];
            if before_arrow.contains(')') && !before_arrow.contains(':') {
                // No return type annotation
                if let Some(paren_pos) = before_arrow.rfind(')') {
                    let col = line.find(')').unwrap_or(0) + 1;
                    hints.push(InlayHint {
                        position: Position::new(line_num, col as u32),
                        label: crate::InlayHintLabel::String(": /* return */".to_string()),
                        kind: crate::InlayHintKind::Type,
                        tooltip: None,
                        padding_left: false,
                        padding_right: true,
                        data: None,
                    });
                }
            }
        }
    }

    hints
}

/// Enum member value hints
pub fn find_enum_value_hints(content: &str) -> Vec<InlayHint> {
    let mut hints = Vec::new();
    let mut in_enum = false;
    let mut current_value = 0i64;

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        
        if trimmed.starts_with("enum ") {
            in_enum = true;
            current_value = 0;
            continue;
        }
        
        if in_enum {
            if trimmed == "}" {
                in_enum = false;
                continue;
            }
            
            // Check for explicit value
            if let Some(eq_pos) = trimmed.find('=') {
                let value_str = &trimmed[eq_pos + 1..].trim();
                if let Some(comma_pos) = value_str.find(',') {
                    if let Ok(v) = value_str[..comma_pos].trim().parse::<i64>() {
                        current_value = v + 1;
                    }
                } else if let Ok(v) = value_str.parse::<i64>() {
                    current_value = v + 1;
                }
            } else if !trimmed.is_empty() && !trimmed.starts_with("//") {
                // No explicit value - show inferred value
                let col = trimmed.find(',').unwrap_or(trimmed.len());
                hints.push(InlayHint {
                    position: Position::new(line_num as u32, col as u32),
                    label: crate::InlayHintLabel::String(format!(" = {}", current_value)),
                    kind: crate::InlayHintKind::Discriminant,
                    tooltip: None,
                    padding_left: false,
                    padding_right: false,
                    data: None,
                });
                current_value += 1;
            }
        }
    }

    hints
}
