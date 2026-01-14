//! Foxkit Terminal - Comprehensive terminal emulation library.
//!
//! This crate provides a full-featured terminal emulator implementation including:
//!
//! - **Emulator**: VT100/xterm escape sequence parser and state machine
//! - **History**: Command history with search and persistence
//! - **Links**: Hyperlink detection (URLs, file paths, OSC 8)
//! - **Profiles**: Terminal profile configuration management
//!
//! # Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────────────┐
//! │                    Terminal                          │
//! │  ┌────────────┐  ┌─────────────┐  ┌──────────────┐  │
//! │  │  Emulator  │  │   History   │  │    Links     │  │
//! │  │  (Parser)  │  │  (Storage)  │  │  (Detector)  │  │
//! │  └─────┬──────┘  └──────┬──────┘  └──────┬───────┘  │
//! │        │                │                │          │
//! │        v                v                v          │
//! │  ┌─────────────────────────────────────────────┐    │
//! │  │              Screen Buffer                  │    │
//! │  │  (Cells, Styles, Scrollback)                │    │
//! │  └─────────────────────────────────────────────┘    │
//! │                        │                            │
//! │        ┌───────────────┼───────────────┐            │
//! │        v               v               v            │
//! │  ┌──────────┐   ┌───────────┐   ┌───────────┐      │
//! │  │ Profiles │   │   PTY     │   │  Render   │      │
//! │  │ (Config) │   │ (Process) │   │  (UI)     │      │
//! │  └──────────┘   └───────────┘   └───────────┘      │
//! └──────────────────────────────────────────────────────┘
//! ```
//!
//! # Quick Start
//!
//! ```rust,no_run
//! use foxkit_terminal::{
//!     emulator::TerminalEmulator,
//!     history::History,
//!     links::LinkDetector,
//!     profiles::{TerminalProfile, ProfileManager},
//! };
//!
//! // Create a terminal emulator
//! let mut emulator = TerminalEmulator::new(80, 24);
//! emulator.process(b"Hello, \x1b[32mWorld\x1b[0m!");
//!
//! // Manage command history
//! let mut history = History::new();
//! history.add_command("cargo build");
//! history.add_command("cargo test");
//! let results = history.search("cargo", history::SearchMode::Substring, 10);
//!
//! // Detect hyperlinks in terminal output
//! let detector = LinkDetector::new();
//! let links = detector.detect_links("Visit https://example.com", 0);
//!
//! // Manage terminal profiles
//! let mut profiles = ProfileManager::new();
//! profiles.add(TerminalProfile::new("dev", "Development Shell")
//!     .with_cwd("/workspace"));
//! ```

pub mod emulator;
pub mod history;
pub mod links;
pub mod profiles;

// Re-export commonly used types
pub use emulator::{
    Cursor, CursorShape, CsiCommand, OscCommand, ParserState, SavedCursor, TerminalEmulator,
    TerminalEvent, TerminalMode,
};
pub use history::{History, HistoryConfig, HistoryEntry, SearchMode, SearchResult, SharedHistory};
pub use links::{
    HyperlinkState, LinkDetector, LinkDetectorConfig, LinkPattern, LinkTarget, TerminalLink,
};
pub use profiles::{
    AnsiColors, BellConfig, ColorScheme, CursorConfig, CursorStyle, FontConfig, ProfileManager,
    RgbColor, ScrollbackConfig, ShellConfig, TerminalProfile, TerminalSize,
};

/// Terminal identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TerminalId(pub u64);

impl TerminalId {
    /// Generate a new unique terminal ID.
    pub fn new() -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for TerminalId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TerminalId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "terminal-{}", self.0)
    }
}

/// A complete terminal instance combining all components.
pub struct Terminal {
    /// Unique identifier.
    pub id: TerminalId,
    /// The terminal emulator.
    pub emulator: TerminalEmulator,
    /// Command history.
    pub history: History,
    /// Link detector.
    pub link_detector: LinkDetector,
    /// The profile being used.
    pub profile: TerminalProfile,
    /// Current title (from OSC sequences).
    title: String,
    /// Whether the terminal is active.
    active: bool,
}

impl Terminal {
    /// Create a new terminal with default settings.
    pub fn new() -> Self {
        let profile = TerminalProfile::default();
        Self::with_profile(profile)
    }

    /// Create a terminal with a specific profile.
    pub fn with_profile(profile: TerminalProfile) -> Self {
        let size = profile.size;
        Self {
            id: TerminalId::new(),
            emulator: TerminalEmulator::new(size.cols as usize, size.rows as usize),
            history: History::new(),
            link_detector: LinkDetector::new(),
            profile,
            title: String::new(),
            active: true,
        }
    }

    /// Process input data through the terminal.
    pub fn process(&mut self, data: &[u8]) {
        self.emulator.process(data);

        // Handle any events
        for event in self.emulator.take_events() {
            match event {
                TerminalEvent::TitleChanged(title) => {
                    self.title = title;
                }
                _ => {}
            }
        }
    }

    /// Write data to the terminal (same as process).
    pub fn write(&mut self, data: &[u8]) {
        self.process(data);
    }

    /// Get the current title.
    pub fn title(&self) -> &str {
        if self.title.is_empty() {
            &self.profile.name
        } else {
            &self.title
        }
    }

    /// Resize the terminal.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        self.emulator.resize(cols as usize, rows as usize);
    }

    /// Reset the terminal to initial state.
    pub fn reset(&mut self) {
        self.emulator.reset();
        self.title.clear();
    }

    /// Check if the terminal is active.
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Set terminal active state.
    pub fn set_active(&mut self, active: bool) {
        self.active = active;
    }

    /// Add a command to history.
    pub fn add_to_history(&mut self, command: impl Into<String>) {
        self.history.add_command(command);
    }

    /// Search command history.
    pub fn search_history(&self, query: &str) -> Vec<SearchResult> {
        self.history.search(query, SearchMode::Substring, 50)
    }

    /// Detect links in a line of terminal output.
    pub fn detect_links(&self, text: &str, row: usize) -> Vec<TerminalLink> {
        self.link_detector.detect_links(text, row)
    }

    /// Get terminal dimensions.
    pub fn size(&self) -> (u16, u16) {
        (self.profile.size.cols, self.profile.size.rows)
    }
}

impl Default for Terminal {
    fn default() -> Self {
        Self::new()
    }
}

/// Manager for multiple terminal instances.
pub struct TerminalManager {
    terminals: std::collections::HashMap<TerminalId, Terminal>,
    active_terminal: Option<TerminalId>,
    profile_manager: ProfileManager,
    shared_history: SharedHistory,
}

impl TerminalManager {
    /// Create a new terminal manager.
    pub fn new() -> Self {
        Self {
            terminals: std::collections::HashMap::new(),
            active_terminal: None,
            profile_manager: ProfileManager::new(),
            shared_history: SharedHistory::new(),
        }
    }

    /// Create a new terminal with the default profile.
    pub fn create(&mut self) -> TerminalId {
        let profile = self
            .profile_manager
            .default_profile()
            .cloned()
            .unwrap_or_default();
        self.create_with_profile(profile)
    }

    /// Create a new terminal with a specific profile.
    pub fn create_with_profile(&mut self, profile: TerminalProfile) -> TerminalId {
        let terminal = Terminal::with_profile(profile);
        let id = terminal.id;

        if self.active_terminal.is_none() {
            self.active_terminal = Some(id);
        }

        self.terminals.insert(id, terminal);
        id
    }

    /// Get a terminal by ID.
    pub fn get(&self, id: TerminalId) -> Option<&Terminal> {
        self.terminals.get(&id)
    }

    /// Get a mutable terminal by ID.
    pub fn get_mut(&mut self, id: TerminalId) -> Option<&mut Terminal> {
        self.terminals.get_mut(&id)
    }

    /// Close a terminal.
    pub fn close(&mut self, id: TerminalId) -> Option<Terminal> {
        let terminal = self.terminals.remove(&id);

        if self.active_terminal == Some(id) {
            self.active_terminal = self.terminals.keys().next().copied();
        }

        terminal
    }

    /// Get the active terminal.
    pub fn active(&self) -> Option<&Terminal> {
        self.active_terminal.and_then(|id| self.terminals.get(&id))
    }

    /// Get the active terminal mutably.
    pub fn active_mut(&mut self) -> Option<&mut Terminal> {
        let id = self.active_terminal?;
        self.terminals.get_mut(&id)
    }

    /// Set the active terminal.
    pub fn set_active(&mut self, id: TerminalId) -> bool {
        if self.terminals.contains_key(&id) {
            self.active_terminal = Some(id);
            true
        } else {
            false
        }
    }

    /// List all terminal IDs.
    pub fn list(&self) -> impl Iterator<Item = TerminalId> + '_ {
        self.terminals.keys().copied()
    }

    /// Get the number of terminals.
    pub fn count(&self) -> usize {
        self.terminals.len()
    }

    /// Access the profile manager.
    pub fn profiles(&self) -> &ProfileManager {
        &self.profile_manager
    }

    /// Access the profile manager mutably.
    pub fn profiles_mut(&mut self) -> &mut ProfileManager {
        &mut self.profile_manager
    }

    /// Access the shared history.
    pub fn shared_history(&self) -> &SharedHistory {
        &self.shared_history
    }
}

impl Default for TerminalManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_terminal_creation() {
        let terminal = Terminal::new();
        assert!(terminal.is_active());
        assert!(!terminal.title().is_empty());
    }

    #[test]
    fn test_terminal_process() {
        let mut terminal = Terminal::new();
        terminal.process(b"Hello, World!");
        // Verify cursor moved
        assert_eq!(terminal.emulator.cursor().col, 13);
    }

    #[test]
    fn test_terminal_title() {
        let mut terminal = Terminal::new();
        terminal.process(b"\x1b]0;My Terminal\x07");
        assert_eq!(terminal.title(), "My Terminal");
    }

    #[test]
    fn test_terminal_manager() {
        let mut manager = TerminalManager::new();

        let id1 = manager.create();
        let id2 = manager.create();

        assert_eq!(manager.count(), 2);
        assert!(manager.active().is_some());

        manager.set_active(id2);
        assert_eq!(manager.active().unwrap().id, id2);

        manager.close(id1);
        assert_eq!(manager.count(), 1);
    }

    #[test]
    fn test_terminal_id() {
        let id1 = TerminalId::new();
        let id2 = TerminalId::new();
        assert_ne!(id1, id2);
    }
}
