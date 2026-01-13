//! Command palette

use crate::{Command, CommandArgs, CommandRegistry, COMMANDS};
use std::sync::Arc;

/// Command palette entry
#[derive(Debug, Clone)]
pub struct PaletteItem {
    /// Command
    pub command: Command,
    /// Recent rank (lower = more recent)
    pub recent_rank: Option<usize>,
    /// Match score
    pub score: f64,
    /// Matched ranges in title
    pub matches: Vec<(usize, usize)>,
}

impl PaletteItem {
    pub fn new(command: Command) -> Self {
        Self {
            command,
            recent_rank: None,
            score: 0.0,
            matches: Vec::new(),
        }
    }

    pub fn with_score(mut self, score: f64) -> Self {
        self.score = score;
        self
    }

    pub fn with_matches(mut self, matches: Vec<(usize, usize)>) -> Self {
        self.matches = matches;
        self
    }
}

/// Command palette
pub struct CommandPalette {
    query: String,
    items: Vec<PaletteItem>,
    selected: usize,
    visible: bool,
}

impl CommandPalette {
    pub fn new() -> Self {
        Self {
            query: String::new(),
            items: Vec::new(),
            selected: 0,
            visible: false,
        }
    }

    /// Show palette
    pub fn show(&mut self) {
        self.visible = true;
        self.query.clear();
        self.selected = 0;
        self.refresh();
    }

    /// Hide palette
    pub fn hide(&mut self) {
        self.visible = false;
    }

    /// Is palette visible
    pub fn is_visible(&self) -> bool {
        self.visible
    }

    /// Get current query
    pub fn query(&self) -> &str {
        &self.query
    }

    /// Set query
    pub fn set_query(&mut self, query: &str) {
        self.query = query.to_string();
        self.refresh();
    }

    /// Update query (append character)
    pub fn push(&mut self, c: char) {
        self.query.push(c);
        self.refresh();
    }

    /// Remove last character
    pub fn pop(&mut self) {
        self.query.pop();
        self.refresh();
    }

    /// Get items
    pub fn items(&self) -> &[PaletteItem] {
        &self.items
    }

    /// Get selected index
    pub fn selected(&self) -> usize {
        self.selected
    }

    /// Get selected item
    pub fn selected_item(&self) -> Option<&PaletteItem> {
        self.items.get(self.selected)
    }

    /// Move selection up
    pub fn select_prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_next(&mut self) {
        if self.selected + 1 < self.items.len() {
            self.selected += 1;
        }
    }

    /// Select by index
    pub fn select(&mut self, index: usize) {
        if index < self.items.len() {
            self.selected = index;
        }
    }

    /// Execute selected command
    pub fn execute_selected(&mut self) -> Option<crate::CommandResult> {
        let item = self.items.get(self.selected)?;
        let id = item.command.id.clone();
        self.hide();
        Some(COMMANDS.run(&id))
    }

    /// Refresh item list
    fn refresh(&mut self) {
        let history = COMMANDS.history();
        let commands = if self.query.is_empty() {
            COMMANDS.visible()
        } else {
            COMMANDS.search(&self.query)
        };

        self.items = commands
            .into_iter()
            .map(|cmd| {
                let recent_rank = history.iter().rev().position(|h| h == &cmd.id);
                let (score, matches) = if self.query.is_empty() {
                    (0.0, Vec::new())
                } else {
                    fuzzy_match(&cmd.full_title(), &self.query)
                };
                
                PaletteItem {
                    command: cmd,
                    recent_rank,
                    score,
                    matches,
                }
            })
            .collect();

        // Sort by score (if query) or recency
        if self.query.is_empty() {
            self.items.sort_by(|a, b| {
                match (a.recent_rank, b.recent_rank) {
                    (Some(ar), Some(br)) => ar.cmp(&br),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.command.title.cmp(&b.command.title),
                }
            });
        } else {
            self.items.sort_by(|a, b| {
                b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)
            });
        }

        // Reset selection
        self.selected = 0;
    }
}

impl Default for CommandPalette {
    fn default() -> Self {
        Self::new()
    }
}

/// Fuzzy match with score and match positions
fn fuzzy_match(text: &str, pattern: &str) -> (f64, Vec<(usize, usize)>) {
    let text_lower = text.to_lowercase();
    let pattern_lower = pattern.to_lowercase();
    
    let mut score = 0.0;
    let mut matches = Vec::new();
    let mut pattern_idx = 0;
    let pattern_chars: Vec<char> = pattern_lower.chars().collect();
    
    if pattern_chars.is_empty() {
        return (0.0, vec![]);
    }

    let mut consecutive = 0;
    let mut prev_match = false;
    
    for (i, c) in text_lower.chars().enumerate() {
        if pattern_idx < pattern_chars.len() && c == pattern_chars[pattern_idx] {
            // Match found
            if prev_match {
                consecutive += 1;
                score += 2.0 * consecutive as f64; // Bonus for consecutive
            } else {
                consecutive = 1;
            }
            
            // Start of word bonus
            if i == 0 || text.chars().nth(i - 1).map(|c| !c.is_alphanumeric()).unwrap_or(true) {
                score += 3.0;
            }
            
            // Case match bonus
            if text.chars().nth(i) == pattern.chars().nth(pattern_idx) {
                score += 0.5;
            }
            
            score += 1.0;
            matches.push((i, i + 1));
            pattern_idx += 1;
            prev_match = true;
        } else {
            prev_match = false;
            consecutive = 0;
        }
    }

    // All pattern chars matched?
    if pattern_idx == pattern_chars.len() {
        // Normalize by length
        score = score / text.len() as f64 * 100.0;
    } else {
        score = 0.0;
        matches.clear();
    }

    (score, matches)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzzy_match() {
        let (score, _) = fuzzy_match("Format Document", "format");
        assert!(score > 0.0);

        let (score, _) = fuzzy_match("Format Document", "fd");
        assert!(score > 0.0);

        let (score, _) = fuzzy_match("Format Document", "xyz");
        assert_eq!(score, 0.0);
    }
}
