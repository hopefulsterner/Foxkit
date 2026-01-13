//! # Foxkit Bracket Matching
//!
//! Highlight and navigate matching brackets, parentheses, and braces.

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Bracket matching service
pub struct BracketMatchingService {
    /// Language bracket configurations
    configs: RwLock<HashMap<String, BracketConfig>>,
    /// Events
    events: broadcast::Sender<BracketEvent>,
    /// Configuration
    config: RwLock<BracketMatchingConfig>,
}

impl BracketMatchingService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);
        let mut configs = HashMap::new();

        // Default configs
        configs.insert("default".to_string(), BracketConfig::default());
        configs.insert("html".to_string(), BracketConfig::html());
        configs.insert("xml".to_string(), BracketConfig::html());

        Self {
            configs: RwLock::new(configs),
            events,
            config: RwLock::new(BracketMatchingConfig::default()),
        }
    }

    /// Register language config
    pub fn register_config(&self, language: impl Into<String>, config: BracketConfig) {
        self.configs.write().insert(language.into(), config);
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<BracketEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: BracketMatchingConfig) {
        *self.config.write() = config;
    }

    /// Find matching bracket
    pub fn find_matching_bracket(
        &self,
        content: &str,
        position: Position,
        language: &str,
    ) -> Option<BracketMatch> {
        let configs = self.configs.read();
        let config = configs.get(language).or_else(|| configs.get("default"))?;

        let chars: Vec<char> = content.chars().collect();
        let offset = position_to_offset(content, &position);

        if offset >= chars.len() {
            return None;
        }

        let char_at = chars[offset];

        // Check if it's an opening bracket
        for pair in &config.pairs {
            if char_at == pair.open {
                let match_pos = find_closing(content, offset, &pair.open, &pair.close)?;
                let match_position = offset_to_position(content, match_pos);

                let _ = self.events.send(BracketEvent::Matched {
                    open: position.clone(),
                    close: match_position.clone(),
                });

                return Some(BracketMatch {
                    bracket: char_at,
                    position,
                    matching_bracket: pair.close,
                    matching_position: match_position,
                    is_opening: true,
                });
            }

            if char_at == pair.close {
                let match_pos = find_opening(content, offset, &pair.open, &pair.close)?;
                let match_position = offset_to_position(content, match_pos);

                let _ = self.events.send(BracketEvent::Matched {
                    open: match_position.clone(),
                    close: position.clone(),
                });

                return Some(BracketMatch {
                    bracket: char_at,
                    position,
                    matching_bracket: pair.open,
                    matching_position: match_position,
                    is_opening: false,
                });
            }
        }

        None
    }

    /// Find all bracket pairs in document
    pub fn find_all_brackets(
        &self,
        content: &str,
        language: &str,
    ) -> Vec<BracketPair> {
        let configs = self.configs.read();
        let config = match configs.get(language).or_else(|| configs.get("default")) {
            Some(c) => c.clone(),
            None => return Vec::new(),
        };
        drop(configs);

        let mut pairs = Vec::new();
        let mut stack: Vec<(char, usize)> = Vec::new();

        for (offset, c) in content.chars().enumerate() {
            for pair in &config.pairs {
                if c == pair.open {
                    stack.push((c, offset));
                } else if c == pair.close {
                    if let Some((open_char, open_offset)) = stack.pop() {
                        if open_char == pair.open {
                            pairs.push(BracketPair {
                                open: offset_to_position(content, open_offset),
                                close: offset_to_position(content, offset),
                                bracket_type: pair.clone(),
                            });
                        }
                    }
                }
            }
        }

        pairs
    }

    /// Get bracket colorization
    pub fn get_bracket_colors(&self, content: &str, language: &str) -> Vec<ColorizedBracket> {
        let config = self.config.read();
        if !config.colorization {
            return Vec::new();
        }

        let pairs = self.find_all_brackets(content, language);
        let colors = &config.colors;
        let mut result = Vec::new();

        // Calculate nesting depth for each bracket
        let mut depths: HashMap<usize, usize> = HashMap::new();
        let mut current_depth = 0;

        for (offset, c) in content.chars().enumerate() {
            if is_opening_bracket(c) {
                depths.insert(offset, current_depth);
                current_depth += 1;
            } else if is_closing_bracket(c) {
                current_depth = current_depth.saturating_sub(1);
                depths.insert(offset, current_depth);
            }
        }

        for pair in pairs {
            let open_offset = position_to_offset(content, &pair.open);
            let close_offset = position_to_offset(content, &pair.close);

            let depth = depths.get(&open_offset).copied().unwrap_or(0);
            let color_index = depth % colors.len();

            result.push(ColorizedBracket {
                position: pair.open,
                bracket: pair.bracket_type.open,
                color: colors[color_index].clone(),
            });

            result.push(ColorizedBracket {
                position: pair.close,
                bracket: pair.bracket_type.close,
                color: colors[color_index].clone(),
            });
        }

        result
    }

    /// Jump to matching bracket
    pub fn jump_to_matching(
        &self,
        content: &str,
        position: Position,
        language: &str,
    ) -> Option<Position> {
        self.find_matching_bracket(content, position, language)
            .map(|m| m.matching_position)
    }

    /// Select to matching bracket
    pub fn select_to_matching(
        &self,
        content: &str,
        position: Position,
        language: &str,
    ) -> Option<Selection> {
        let match_result = self.find_matching_bracket(content, position.clone(), language)?;

        let (start, end) = if match_result.is_opening {
            (position, match_result.matching_position)
        } else {
            (match_result.matching_position, position)
        };

        Some(Selection {
            start_line: start.line,
            start_col: start.col,
            end_line: end.line,
            end_col: end.col + 1, // Include the bracket
        })
    }
}

impl Default for BracketMatchingService {
    fn default() -> Self {
        Self::new()
    }
}

fn is_opening_bracket(c: char) -> bool {
    matches!(c, '(' | '[' | '{' | '<')
}

fn is_closing_bracket(c: char) -> bool {
    matches!(c, ')' | ']' | '}' | '>')
}

fn position_to_offset(content: &str, position: &Position) -> usize {
    let mut offset = 0;
    
    for (i, line) in content.lines().enumerate() {
        if i == position.line as usize {
            return offset + position.col as usize;
        }
        offset += line.len() + 1; // +1 for newline
    }

    offset
}

fn offset_to_position(content: &str, offset: usize) -> Position {
    let mut current_offset = 0;
    
    for (line_num, line) in content.lines().enumerate() {
        let line_end = current_offset + line.len();
        
        if offset <= line_end {
            return Position {
                line: line_num as u32,
                col: (offset - current_offset) as u32,
            };
        }
        
        current_offset = line_end + 1; // +1 for newline
    }

    Position { line: 0, col: 0 }
}

fn find_closing(content: &str, start: usize, open: &char, close: &char) -> Option<usize> {
    let chars: Vec<char> = content.chars().collect();
    let mut depth = 1;

    for i in (start + 1)..chars.len() {
        if chars[i] == *open {
            depth += 1;
        } else if chars[i] == *close {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }

    None
}

fn find_opening(content: &str, start: usize, open: &char, close: &char) -> Option<usize> {
    let chars: Vec<char> = content.chars().collect();
    let mut depth = 1;

    for i in (0..start).rev() {
        if chars[i] == *close {
            depth += 1;
        } else if chars[i] == *open {
            depth -= 1;
            if depth == 0 {
                return Some(i);
            }
        }
    }

    None
}

/// Position
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub col: u32,
}

impl Position {
    pub fn new(line: u32, col: u32) -> Self {
        Self { line, col }
    }
}

/// Selection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Selection {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// Bracket configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketConfig {
    pub pairs: Vec<BracketType>,
}

impl BracketConfig {
    pub fn new() -> Self {
        Self { pairs: Vec::new() }
    }

    pub fn with_pair(mut self, open: char, close: char) -> Self {
        self.pairs.push(BracketType { open, close });
        self
    }

    /// HTML/XML config with angle brackets
    pub fn html() -> Self {
        Self::default().with_pair('<', '>')
    }
}

impl Default for BracketConfig {
    fn default() -> Self {
        Self::new()
            .with_pair('(', ')')
            .with_pair('[', ']')
            .with_pair('{', '}')
    }
}

/// Bracket type
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketType {
    pub open: char,
    pub close: char,
}

/// Bracket match result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketMatch {
    pub bracket: char,
    pub position: Position,
    pub matching_bracket: char,
    pub matching_position: Position,
    pub is_opening: bool,
}

/// Bracket pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketPair {
    pub open: Position,
    pub close: Position,
    pub bracket_type: BracketType,
}

/// Colorized bracket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorizedBracket {
    pub position: Position,
    pub bracket: char,
    pub color: String,
}

/// Bracket matching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BracketMatchingConfig {
    /// Enable bracket matching
    pub enabled: bool,
    /// Highlight matching bracket
    pub highlight: bool,
    /// Bracket pair colorization
    pub colorization: bool,
    /// Colors for nested brackets
    pub colors: Vec<String>,
    /// Colorization in independent pairs
    pub independent_color_pool: bool,
}

impl Default for BracketMatchingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            highlight: true,
            colorization: true,
            colors: vec![
                "#FFD700".to_string(), // Gold
                "#DA70D6".to_string(), // Orchid
                "#179FFF".to_string(), // Blue
            ],
            independent_color_pool: false,
        }
    }
}

/// Bracket event
#[derive(Debug, Clone)]
pub enum BracketEvent {
    Matched { open: Position, close: Position },
    NoMatch { position: Position },
}
