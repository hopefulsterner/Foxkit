//! Slash Commands for AI Chat
//!
//! Commands like /explain, /fix, /generate, /refactor that provide
//! structured interactions with the AI.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A slash command that can be used in chat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommand {
    /// Command name (without the /)
    pub name: String,
    /// Short description
    pub description: String,
    /// Long help text
    pub help: String,
    /// Parameters this command accepts
    pub parameters: Vec<SlashCommandParam>,
    /// Whether this command requires selection
    pub requires_selection: bool,
    /// Whether this command requires active file
    pub requires_file: bool,
    /// Example usages
    pub examples: Vec<String>,
}

/// Parameter for a slash command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlashCommandParam {
    pub name: String,
    pub description: String,
    pub required: bool,
    pub param_type: ParamType,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamType {
    Text,
    File,
    Symbol,
    Language,
    Number,
    Boolean,
    Choice(Vec<String>),
}

/// Parsed slash command invocation
#[derive(Debug, Clone)]
pub struct ParsedCommand {
    pub name: String,
    pub arguments: HashMap<String, String>,
    pub raw_input: String,
}

/// Slash command registry
pub struct SlashCommandRegistry {
    commands: HashMap<String, SlashCommand>,
}

impl SlashCommandRegistry {
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };
        
        // Register built-in commands
        registry.register_builtin_commands();
        registry
    }

    fn register_builtin_commands(&mut self) {
        // /explain - Explain code
        self.register(SlashCommand {
            name: "explain".into(),
            description: "Explain the selected code or concept".into(),
            help: "Get a detailed explanation of what the selected code does, or explain a programming concept.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "detail".into(),
                    description: "Level of detail (brief, normal, detailed)".into(),
                    required: false,
                    param_type: ParamType::Choice(vec!["brief".into(), "normal".into(), "detailed".into()]),
                    default: Some("normal".into()),
                },
            ],
            requires_selection: false,
            requires_file: false,
            examples: vec![
                "/explain".into(),
                "/explain detail:detailed".into(),
            ],
        });

        // /fix - Fix code issues
        self.register(SlashCommand {
            name: "fix".into(),
            description: "Fix errors in the selected code".into(),
            help: "Analyze and fix bugs, errors, or issues in the selected code.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "error".into(),
                    description: "Specific error message to fix".into(),
                    required: false,
                    param_type: ParamType::Text,
                    default: None,
                },
            ],
            requires_selection: true,
            requires_file: true,
            examples: vec![
                "/fix".into(),
                "/fix error:\"undefined variable\"".into(),
            ],
        });

        // /refactor - Refactor code
        self.register(SlashCommand {
            name: "refactor".into(),
            description: "Refactor the selected code".into(),
            help: "Improve code structure, readability, or performance without changing behavior.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "goal".into(),
                    description: "Refactoring goal (readability, performance, dry, solid)".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "readability".into(),
                        "performance".into(),
                        "dry".into(),
                        "solid".into(),
                        "testable".into(),
                    ]),
                    default: Some("readability".into()),
                },
            ],
            requires_selection: true,
            requires_file: true,
            examples: vec![
                "/refactor".into(),
                "/refactor goal:performance".into(),
            ],
        });

        // /generate - Generate code
        self.register(SlashCommand {
            name: "generate".into(),
            description: "Generate new code".into(),
            help: "Generate code based on a description or specification.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "type".into(),
                    description: "Type of code to generate".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "function".into(),
                        "class".into(),
                        "test".into(),
                        "component".into(),
                        "api".into(),
                    ]),
                    default: None,
                },
                SlashCommandParam {
                    name: "language".into(),
                    description: "Programming language".into(),
                    required: false,
                    param_type: ParamType::Language,
                    default: None,
                },
            ],
            requires_selection: false,
            requires_file: false,
            examples: vec![
                "/generate a function that sorts an array".into(),
                "/generate type:test for the selected function".into(),
            ],
        });

        // /doc - Generate documentation
        self.register(SlashCommand {
            name: "doc".into(),
            description: "Generate documentation".into(),
            help: "Generate documentation comments for the selected code.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "style".into(),
                    description: "Documentation style".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "jsdoc".into(),
                        "rustdoc".into(),
                        "pydoc".into(),
                        "javadoc".into(),
                        "markdown".into(),
                    ]),
                    default: None,
                },
            ],
            requires_selection: true,
            requires_file: true,
            examples: vec![
                "/doc".into(),
                "/doc style:rustdoc".into(),
            ],
        });

        // /test - Generate tests
        self.register(SlashCommand {
            name: "test".into(),
            description: "Generate unit tests".into(),
            help: "Generate unit tests for the selected code or function.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "framework".into(),
                    description: "Testing framework to use".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "jest".into(),
                        "mocha".into(),
                        "pytest".into(),
                        "rust".into(),
                        "junit".into(),
                        "vitest".into(),
                    ]),
                    default: None,
                },
                SlashCommandParam {
                    name: "coverage".into(),
                    description: "Coverage level (basic, thorough, edge-cases)".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "basic".into(),
                        "thorough".into(),
                        "edge-cases".into(),
                    ]),
                    default: Some("thorough".into()),
                },
            ],
            requires_selection: true,
            requires_file: true,
            examples: vec![
                "/test".into(),
                "/test framework:jest coverage:edge-cases".into(),
            ],
        });

        // /review - Code review
        self.register(SlashCommand {
            name: "review".into(),
            description: "Review code for issues".into(),
            help: "Perform a code review looking for bugs, security issues, and improvements.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "focus".into(),
                    description: "Review focus area".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "bugs".into(),
                        "security".into(),
                        "performance".into(),
                        "style".into(),
                        "all".into(),
                    ]),
                    default: Some("all".into()),
                },
            ],
            requires_selection: false,
            requires_file: true,
            examples: vec![
                "/review".into(),
                "/review focus:security".into(),
            ],
        });

        // /optimize - Optimize code
        self.register(SlashCommand {
            name: "optimize".into(),
            description: "Optimize code performance".into(),
            help: "Analyze and optimize code for better performance.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "target".into(),
                    description: "Optimization target".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "speed".into(),
                        "memory".into(),
                        "bundle-size".into(),
                        "all".into(),
                    ]),
                    default: Some("speed".into()),
                },
            ],
            requires_selection: true,
            requires_file: true,
            examples: vec![
                "/optimize".into(),
                "/optimize target:memory".into(),
            ],
        });

        // /convert - Convert between formats/languages
        self.register(SlashCommand {
            name: "convert".into(),
            description: "Convert code to another language/format".into(),
            help: "Convert selected code to a different programming language or format.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "to".into(),
                    description: "Target language or format".into(),
                    required: true,
                    param_type: ParamType::Language,
                    default: None,
                },
            ],
            requires_selection: true,
            requires_file: false,
            examples: vec![
                "/convert to:typescript".into(),
                "/convert to:python".into(),
            ],
        });

        // /summarize - Summarize code/file
        self.register(SlashCommand {
            name: "summarize".into(),
            description: "Summarize code or file".into(),
            help: "Get a high-level summary of what the code/file does.".into(),
            parameters: vec![],
            requires_selection: false,
            requires_file: true,
            examples: vec![
                "/summarize".into(),
            ],
        });

        // /deps - Analyze dependencies
        self.register(SlashCommand {
            name: "deps".into(),
            description: "Analyze dependencies".into(),
            help: "Analyze the dependencies of the current file or selection.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "direction".into(),
                    description: "Dependency direction".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "imports".into(),
                        "exports".into(),
                        "both".into(),
                    ]),
                    default: Some("both".into()),
                },
            ],
            requires_selection: false,
            requires_file: true,
            examples: vec![
                "/deps".into(),
                "/deps direction:imports".into(),
            ],
        });

        // /ask - General question
        self.register(SlashCommand {
            name: "ask".into(),
            description: "Ask a question about the code".into(),
            help: "Ask any question about the codebase, file, or selection.".into(),
            parameters: vec![],
            requires_selection: false,
            requires_file: false,
            examples: vec![
                "/ask how does the auth system work?".into(),
                "/ask what does this function do?".into(),
            ],
        });

        // /workspace - Workspace context
        self.register(SlashCommand {
            name: "workspace".into(),
            description: "Include workspace context".into(),
            help: "Include broader workspace context in the AI's understanding.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "scope".into(),
                    description: "Context scope".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "package".into(),
                        "directory".into(),
                        "related".into(),
                        "full".into(),
                    ]),
                    default: Some("related".into()),
                },
            ],
            requires_selection: false,
            requires_file: false,
            examples: vec![
                "/workspace".into(),
                "/workspace scope:package".into(),
            ],
        });

        // /commit - Generate commit message
        self.register(SlashCommand {
            name: "commit".into(),
            description: "Generate a commit message".into(),
            help: "Generate a conventional commit message for staged changes.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "style".into(),
                    description: "Commit style".into(),
                    required: false,
                    param_type: ParamType::Choice(vec![
                        "conventional".into(),
                        "gitmoji".into(),
                        "simple".into(),
                    ]),
                    default: Some("conventional".into()),
                },
            ],
            requires_selection: false,
            requires_file: false,
            examples: vec![
                "/commit".into(),
                "/commit style:gitmoji".into(),
            ],
        });

        // /mcp - Use MCP tool
        self.register(SlashCommand {
            name: "mcp".into(),
            description: "Call an MCP server tool".into(),
            help: "Invoke a tool from a connected MCP server.".into(),
            parameters: vec![
                SlashCommandParam {
                    name: "server".into(),
                    description: "MCP server name".into(),
                    required: false,
                    param_type: ParamType::Text,
                    default: None,
                },
                SlashCommandParam {
                    name: "tool".into(),
                    description: "Tool name".into(),
                    required: true,
                    param_type: ParamType::Text,
                    default: None,
                },
            ],
            requires_selection: false,
            requires_file: false,
            examples: vec![
                "/mcp tool:search_files".into(),
                "/mcp server:filesystem tool:read_file".into(),
            ],
        });
    }

    /// Register a custom slash command
    pub fn register(&mut self, command: SlashCommand) {
        self.commands.insert(command.name.clone(), command);
    }

    /// Get a command by name
    pub fn get(&self, name: &str) -> Option<&SlashCommand> {
        self.commands.get(name)
    }

    /// List all commands
    pub fn list(&self) -> Vec<&SlashCommand> {
        self.commands.values().collect()
    }

    /// Parse a slash command from input
    pub fn parse(&self, input: &str) -> Option<ParsedCommand> {
        let input = input.trim();
        if !input.starts_with('/') {
            return None;
        }

        let parts: Vec<&str> = input[1..].splitn(2, |c: char| c.is_whitespace()).collect();
        let name = parts.first()?.to_lowercase();
        
        // Check if command exists
        self.get(&name)?;

        let raw_input = parts.get(1).unwrap_or(&"").to_string();
        let arguments = self.parse_arguments(&raw_input);

        Some(ParsedCommand {
            name,
            arguments,
            raw_input,
        })
    }

    fn parse_arguments(&self, input: &str) -> HashMap<String, String> {
        let mut args = HashMap::new();
        
        // Parse key:value pairs
        let mut in_quotes = false;
        let mut current_key = String::new();
        let mut current_value = String::new();
        let mut parsing_value = false;

        for c in input.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ':' if !in_quotes && !parsing_value => {
                    parsing_value = true;
                }
                ' ' if !in_quotes => {
                    if !current_key.is_empty() {
                        args.insert(
                            current_key.trim().to_string(),
                            current_value.trim().to_string(),
                        );
                    }
                    current_key.clear();
                    current_value.clear();
                    parsing_value = false;
                }
                _ => {
                    if parsing_value {
                        current_value.push(c);
                    } else {
                        current_key.push(c);
                    }
                }
            }
        }

        // Don't forget the last argument
        if !current_key.is_empty() {
            args.insert(
                current_key.trim().to_string(),
                current_value.trim().to_string(),
            );
        }

        args
    }

    /// Get completions for a partial command
    pub fn completions(&self, partial: &str) -> Vec<&SlashCommand> {
        if !partial.starts_with('/') {
            return vec![];
        }

        let search = partial[1..].to_lowercase();
        self.commands
            .values()
            .filter(|cmd| cmd.name.starts_with(&search))
            .collect()
    }
}

impl Default for SlashCommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Build system prompt for a slash command
pub fn build_command_prompt(command: &ParsedCommand, context: &CommandContext) -> String {
    let mut prompt = String::new();

    match command.name.as_str() {
        "explain" => {
            let detail = command.arguments.get("detail").map(|s| s.as_str()).unwrap_or("normal");
            prompt.push_str(&format!(
                "Explain the following code in {} detail.\n\n",
                detail
            ));
            if let Some(selection) = &context.selection {
                prompt.push_str(&format!("```{}\n{}\n```\n", 
                    context.language.as_deref().unwrap_or(""),
                    selection
                ));
            } else {
                prompt.push_str(&command.raw_input);
            }
        }
        "fix" => {
            prompt.push_str("Fix the following code. ");
            if let Some(error) = command.arguments.get("error") {
                prompt.push_str(&format!("The error is: {}\n\n", error));
            }
            if let Some(selection) = &context.selection {
                prompt.push_str(&format!("```{}\n{}\n```\n",
                    context.language.as_deref().unwrap_or(""),
                    selection
                ));
            }
            prompt.push_str("\nProvide the corrected code with an explanation of what was wrong.");
        }
        "refactor" => {
            let goal = command.arguments.get("goal").map(|s| s.as_str()).unwrap_or("readability");
            prompt.push_str(&format!(
                "Refactor the following code to improve {}.\n\n",
                goal
            ));
            if let Some(selection) = &context.selection {
                prompt.push_str(&format!("```{}\n{}\n```\n",
                    context.language.as_deref().unwrap_or(""),
                    selection
                ));
            }
            prompt.push_str("\nProvide the refactored code with explanations for each change.");
        }
        "generate" => {
            let type_hint = command.arguments.get("type").map(|s| format!(" ({})", s)).unwrap_or_default();
            let lang = command.arguments.get("language").or(context.language.as_ref());
            prompt.push_str(&format!(
                "Generate{} code{}:\n\n{}\n",
                type_hint,
                lang.map(|l| format!(" in {}", l)).unwrap_or_default(),
                command.raw_input
            ));
        }
        "doc" => {
            let style = command.arguments.get("style")
                .or(context.language.as_ref().map(|l| match l.as_str() {
                    "rust" => "rustdoc",
                    "python" => "pydoc",
                    "javascript" | "typescript" => "jsdoc",
                    "java" | "kotlin" => "javadoc",
                    _ => "markdown"
                }).map(String::from).as_ref())
                .map(|s| s.as_str())
                .unwrap_or("markdown");
            
            prompt.push_str(&format!(
                "Generate {} documentation for the following code:\n\n",
                style
            ));
            if let Some(selection) = &context.selection {
                prompt.push_str(&format!("```{}\n{}\n```\n",
                    context.language.as_deref().unwrap_or(""),
                    selection
                ));
            }
        }
        "test" => {
            let framework = command.arguments.get("framework")
                .or(context.language.as_ref().map(|l| match l.as_str() {
                    "rust" => "rust",
                    "python" => "pytest",
                    "javascript" | "typescript" => "jest",
                    "java" => "junit",
                    _ => "jest"
                }).map(String::from).as_ref())
                .map(|s| s.as_str())
                .unwrap_or("jest");
            let coverage = command.arguments.get("coverage").map(|s| s.as_str()).unwrap_or("thorough");
            
            prompt.push_str(&format!(
                "Generate {} unit tests for the following code with {} coverage:\n\n",
                framework, coverage
            ));
            if let Some(selection) = &context.selection {
                prompt.push_str(&format!("```{}\n{}\n```\n",
                    context.language.as_deref().unwrap_or(""),
                    selection
                ));
            }
        }
        "review" => {
            let focus = command.arguments.get("focus").map(|s| s.as_str()).unwrap_or("all");
            prompt.push_str(&format!(
                "Review the following code focusing on {}:\n\n",
                focus
            ));
            if let Some(file_content) = &context.file_content {
                prompt.push_str(&format!("```{}\n{}\n```\n",
                    context.language.as_deref().unwrap_or(""),
                    file_content
                ));
            }
            prompt.push_str("\nProvide specific feedback with line numbers and suggested improvements.");
        }
        "commit" => {
            let style = command.arguments.get("style").map(|s| s.as_str()).unwrap_or("conventional");
            prompt.push_str(&format!(
                "Generate a {} commit message for the following changes:\n\n",
                style
            ));
            if let Some(diff) = &context.git_diff {
                prompt.push_str(&format!("```diff\n{}\n```\n", diff));
            }
        }
        _ => {
            // Generic handling
            prompt.push_str(&command.raw_input);
        }
    }

    prompt
}

/// Context available for command execution
#[derive(Debug, Clone, Default)]
pub struct CommandContext {
    pub file_path: Option<String>,
    pub language: Option<String>,
    pub selection: Option<String>,
    pub file_content: Option<String>,
    pub git_diff: Option<String>,
    pub workspace_root: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_command() {
        let registry = SlashCommandRegistry::new();
        let parsed = registry.parse("/explain").unwrap();
        assert_eq!(parsed.name, "explain");
    }

    #[test]
    fn test_parse_command_with_args() {
        let registry = SlashCommandRegistry::new();
        let parsed = registry.parse("/refactor goal:performance").unwrap();
        assert_eq!(parsed.name, "refactor");
        assert_eq!(parsed.arguments.get("goal"), Some(&"performance".to_string()));
    }

    #[test]
    fn test_completions() {
        let registry = SlashCommandRegistry::new();
        let completions = registry.completions("/re");
        assert!(completions.iter().any(|c| c.name == "review"));
        assert!(completions.iter().any(|c| c.name == "refactor"));
    }
}
