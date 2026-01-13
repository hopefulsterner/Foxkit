//! # Foxkit Multi-Cursor
//!
//! Multiple cursor editing and selection management.

use std::collections::HashSet;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;

/// Multi-cursor service
pub struct MultiCursorService {
    /// Cursors state
    state: RwLock<MultiCursorState>,
    /// Events
    events: broadcast::Sender<MultiCursorEvent>,
    /// Configuration
    config: RwLock<MultiCursorConfig>,
}

impl MultiCursorService {
    pub fn new() -> Self {
        let (events, _) = broadcast::channel(64);

        Self {
            state: RwLock::new(MultiCursorState::new()),
            events,
            config: RwLock::new(MultiCursorConfig::default()),
        }
    }

    /// Subscribe to events
    pub fn subscribe(&self) -> broadcast::Receiver<MultiCursorEvent> {
        self.events.subscribe()
    }

    /// Configure service
    pub fn configure(&self, config: MultiCursorConfig) {
        *self.config.write() = config;
    }

    /// Get all cursors
    pub fn get_cursors(&self) -> Vec<Cursor> {
        self.state.read().cursors.clone()
    }

    /// Get primary cursor
    pub fn get_primary(&self) -> Option<Cursor> {
        let state = self.state.read();
        state.cursors.get(state.primary).cloned()
    }

    /// Set single cursor
    pub fn set_cursor(&self, position: Position) {
        let mut state = self.state.write();
        state.cursors = vec![Cursor::new(position)];
        state.primary = 0;

        let _ = self.events.send(MultiCursorEvent::Changed {
            count: 1,
        });
    }

    /// Add cursor
    pub fn add_cursor(&self, position: Position) {
        let mut state = self.state.write();
        
        // Check if cursor already exists at position
        if state.cursors.iter().any(|c| c.position == position) {
            return;
        }

        state.cursors.push(Cursor::new(position));

        let _ = self.events.send(MultiCursorEvent::Added {
            position: position.clone(),
            count: state.cursors.len(),
        });
    }

    /// Add cursor above
    pub fn add_cursor_above(&self) {
        let mut state = self.state.write();
        
        if let Some(primary) = state.cursors.get(state.primary) {
            if primary.position.line > 0 {
                let new_pos = Position {
                    line: primary.position.line - 1,
                    col: primary.position.col,
                };
                
                if !state.cursors.iter().any(|c| c.position == new_pos) {
                    state.cursors.push(Cursor::new(new_pos.clone()));
                    
                    let _ = self.events.send(MultiCursorEvent::Added {
                        position: new_pos,
                        count: state.cursors.len(),
                    });
                }
            }
        }
    }

    /// Add cursor below
    pub fn add_cursor_below(&self) {
        let mut state = self.state.write();
        
        if let Some(primary) = state.cursors.get(state.primary) {
            let new_pos = Position {
                line: primary.position.line + 1,
                col: primary.position.col,
            };
            
            if !state.cursors.iter().any(|c| c.position == new_pos) {
                state.cursors.push(Cursor::new(new_pos.clone()));
                
                let _ = self.events.send(MultiCursorEvent::Added {
                    position: new_pos,
                    count: state.cursors.len(),
                });
            }
        }
    }

    /// Remove cursor at position
    pub fn remove_cursor(&self, position: &Position) {
        let mut state = self.state.write();
        
        if state.cursors.len() <= 1 {
            return;
        }

        if let Some(idx) = state.cursors.iter().position(|c| &c.position == position) {
            state.cursors.remove(idx);
            
            if state.primary >= state.cursors.len() {
                state.primary = state.cursors.len() - 1;
            }

            let _ = self.events.send(MultiCursorEvent::Removed {
                position: position.clone(),
                count: state.cursors.len(),
            });
        }
    }

    /// Remove secondary cursors
    pub fn remove_secondary(&self) {
        let mut state = self.state.write();
        
        if state.cursors.len() <= 1 {
            return;
        }

        let primary = state.cursors.get(state.primary).cloned();
        
        if let Some(cursor) = primary {
            state.cursors = vec![cursor];
            state.primary = 0;

            let _ = self.events.send(MultiCursorEvent::Changed { count: 1 });
        }
    }

    /// Set selection for cursor
    pub fn set_selection(&self, cursor_index: usize, selection: Selection) {
        let mut state = self.state.write();
        
        if let Some(cursor) = state.cursors.get_mut(cursor_index) {
            cursor.selection = Some(selection);
        }
    }

    /// Add cursors for all occurrences of word
    pub fn add_cursors_for_word(&self, content: &str, word: &str) {
        let mut state = self.state.write();
        
        for (line_num, line) in content.lines().enumerate() {
            let mut col = 0;
            for part in line.split(word) {
                col += part.len();
                if col < line.len() {
                    let pos = Position {
                        line: line_num as u32,
                        col: col as u32,
                    };
                    
                    if !state.cursors.iter().any(|c| c.position == pos) {
                        state.cursors.push(Cursor::new_with_selection(
                            pos.clone(),
                            Selection {
                                start: pos.clone(),
                                end: Position {
                                    line: line_num as u32,
                                    col: (col + word.len()) as u32,
                                },
                            },
                        ));
                    }
                    
                    col += word.len();
                }
            }
        }

        let _ = self.events.send(MultiCursorEvent::Changed {
            count: state.cursors.len(),
        });
    }

    /// Move all cursors
    pub fn move_all(&self, direction: Direction, by: MoveBy) {
        let mut state = self.state.write();
        
        for cursor in &mut state.cursors {
            cursor.move_cursor(direction, &by);
        }

        // Merge overlapping cursors
        Self::merge_overlapping(&mut state.cursors);

        let _ = self.events.send(MultiCursorEvent::Moved {
            count: state.cursors.len(),
        });
    }

    /// Merge overlapping cursors
    fn merge_overlapping(cursors: &mut Vec<Cursor>) {
        let mut seen = HashSet::new();
        cursors.retain(|c| seen.insert((c.position.line, c.position.col)));
    }

    /// Type at all cursors
    pub fn type_text(&self, text: &str) -> Vec<TypeEdit> {
        let state = self.state.read();
        
        state.cursors
            .iter()
            .map(|cursor| TypeEdit {
                position: cursor.position.clone(),
                text: text.to_string(),
            })
            .collect()
    }

    /// Delete at all cursors
    pub fn delete(&self, direction: DeleteDirection) -> Vec<DeleteEdit> {
        let state = self.state.read();
        
        state.cursors
            .iter()
            .map(|cursor| DeleteEdit {
                position: cursor.position.clone(),
                direction,
            })
            .collect()
    }

    /// Get cursor count
    pub fn cursor_count(&self) -> usize {
        self.state.read().cursors.len()
    }

    /// Is multi-cursor mode active
    pub fn is_multi_cursor(&self) -> bool {
        self.state.read().cursors.len() > 1
    }
}

impl Default for MultiCursorService {
    fn default() -> Self {
        Self::new()
    }
}

/// Multi-cursor state
struct MultiCursorState {
    /// All cursors
    cursors: Vec<Cursor>,
    /// Primary cursor index
    primary: usize,
}

impl MultiCursorState {
    fn new() -> Self {
        Self {
            cursors: vec![Cursor::new(Position { line: 0, col: 0 })],
            primary: 0,
        }
    }
}

/// Cursor
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cursor {
    /// Position
    pub position: Position,
    /// Selection
    pub selection: Option<Selection>,
    /// Preferred column (for vertical movement)
    pub preferred_col: Option<u32>,
}

impl Cursor {
    pub fn new(position: Position) -> Self {
        Self {
            position,
            selection: None,
            preferred_col: None,
        }
    }

    pub fn new_with_selection(position: Position, selection: Selection) -> Self {
        Self {
            position,
            selection: Some(selection),
            preferred_col: None,
        }
    }

    pub fn move_cursor(&mut self, direction: Direction, by: &MoveBy) {
        match direction {
            Direction::Left => {
                if self.position.col > 0 {
                    self.position.col -= 1;
                }
            }
            Direction::Right => {
                self.position.col += 1;
            }
            Direction::Up => {
                if self.position.line > 0 {
                    self.position.line -= 1;
                    if let Some(col) = self.preferred_col {
                        self.position.col = col;
                    }
                }
            }
            Direction::Down => {
                self.position.line += 1;
                if let Some(col) = self.preferred_col {
                    self.position.col = col;
                }
            }
        }
    }

    pub fn has_selection(&self) -> bool {
        self.selection.is_some()
    }
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
    pub start: Position,
    pub end: Position,
}

impl Selection {
    pub fn new(start: Position, end: Position) -> Self {
        Self { start, end }
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }

    pub fn is_reversed(&self) -> bool {
        self.start.line > self.end.line ||
            (self.start.line == self.end.line && self.start.col > self.end.col)
    }

    pub fn normalized(&self) -> Self {
        if self.is_reversed() {
            Self {
                start: self.end.clone(),
                end: self.start.clone(),
            }
        } else {
            self.clone()
        }
    }
}

/// Direction
#[derive(Debug, Clone, Copy)]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

/// Move by unit
#[derive(Debug, Clone, Copy)]
pub enum MoveBy {
    Character,
    Word,
    Line,
    Page,
    Document,
}

/// Delete direction
#[derive(Debug, Clone, Copy)]
pub enum DeleteDirection {
    Forward,
    Backward,
}

/// Type edit
#[derive(Debug, Clone)]
pub struct TypeEdit {
    pub position: Position,
    pub text: String,
}

/// Delete edit
#[derive(Debug, Clone)]
pub struct DeleteEdit {
    pub position: Position,
    pub direction: DeleteDirection,
}

/// Multi-cursor configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiCursorConfig {
    /// Modifier key for adding cursors
    pub modifier: CursorModifier,
    /// Paste behavior with multiple cursors
    pub paste_behavior: MultiCursorPaste,
    /// Limit number of cursors
    pub max_cursors: usize,
}

impl Default for MultiCursorConfig {
    fn default() -> Self {
        Self {
            modifier: CursorModifier::Alt,
            paste_behavior: MultiCursorPaste::Spread,
            max_cursors: 10000,
        }
    }
}

/// Cursor modifier key
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CursorModifier {
    Alt,
    Ctrl,
    Meta,
}

/// Multi-cursor paste behavior
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MultiCursorPaste {
    /// Paste full text at each cursor
    Full,
    /// Spread lines across cursors
    Spread,
}

/// Multi-cursor event
#[derive(Debug, Clone)]
pub enum MultiCursorEvent {
    Added { position: Position, count: usize },
    Removed { position: Position, count: usize },
    Changed { count: usize },
    Moved { count: usize },
}
