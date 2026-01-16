//! LSP Workspace Management
//!
//! Multi-root workspace support and file watching.

use lsp_types::*;
use std::collections::HashMap;
use parking_lot::RwLock;

/// Workspace folder management
pub struct WorkspaceManager {
    /// Active workspace folders
    folders: RwLock<Vec<WorkspaceFolder>>,
    /// File watchers per folder
    watchers: RwLock<HashMap<String, Vec<FileWatcher>>>,
    /// Document version tracking
    document_versions: RwLock<HashMap<Url, i32>>,
    /// Open documents
    open_documents: RwLock<HashMap<Url, TextDocument>>,
}

/// Tracked text document
#[derive(Debug, Clone)]
pub struct TextDocument {
    pub uri: Url,
    pub language_id: String,
    pub version: i32,
    pub content: String,
}

impl TextDocument {
    pub fn new(uri: Url, language_id: String, version: i32, content: String) -> Self {
        Self { uri, language_id, version, content }
    }

    /// Apply incremental changes
    pub fn apply_changes(&mut self, changes: Vec<TextDocumentContentChangeEvent>) {
        for change in changes {
            if let Some(range) = change.range {
                // Incremental update
                let start_offset = self.offset_at(range.start);
                let end_offset = self.offset_at(range.end);
                
                let mut new_content = String::new();
                new_content.push_str(&self.content[..start_offset]);
                new_content.push_str(&change.text);
                new_content.push_str(&self.content[end_offset..]);
                self.content = new_content;
            } else {
                // Full replacement
                self.content = change.text;
            }
        }
        self.version += 1;
    }

    /// Get offset from position
    pub fn offset_at(&self, position: Position) -> usize {
        let mut offset = 0;
        let mut line = 0;
        
        for ch in self.content.chars() {
            if line == position.line {
                let mut col = 0;
                for ch in self.content[offset..].chars() {
                    if col == position.character {
                        return offset;
                    }
                    offset += ch.len_utf8();
                    col += 1;
                }
                return offset;
            }
            
            if ch == '\n' {
                line += 1;
            }
            offset += ch.len_utf8();
        }
        
        offset
    }

    /// Get position from offset
    pub fn position_at(&self, offset: usize) -> Position {
        let mut line = 0;
        let mut character = 0;
        let mut current_offset = 0;
        
        for ch in self.content.chars() {
            if current_offset >= offset {
                break;
            }
            
            if ch == '\n' {
                line += 1;
                character = 0;
            } else {
                character += 1;
            }
            current_offset += ch.len_utf8();
        }
        
        Position { line, character }
    }

    /// Get line count
    pub fn line_count(&self) -> u32 {
        self.content.lines().count() as u32
    }

    /// Get text in range
    pub fn get_text(&self, range: Range) -> &str {
        let start = self.offset_at(range.start);
        let end = self.offset_at(range.end);
        &self.content[start..end]
    }

    /// Get line content
    pub fn get_line(&self, line: u32) -> Option<&str> {
        self.content.lines().nth(line as usize)
    }

    /// Get word at position
    pub fn get_word_at(&self, position: Position) -> Option<String> {
        let line = self.get_line(position.line)?;
        let col = position.character as usize;
        
        if col >= line.len() {
            return None;
        }
        
        let chars: Vec<char> = line.chars().collect();
        let mut start = col;
        let mut end = col;
        
        // Find word boundaries
        while start > 0 && is_word_char(chars[start - 1]) {
            start -= 1;
        }
        while end < chars.len() && is_word_char(chars[end]) {
            end += 1;
        }
        
        if start == end {
            None
        } else {
            Some(chars[start..end].iter().collect())
        }
    }
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// File watcher for workspace changes
#[derive(Debug, Clone)]
pub struct FileWatcher {
    pub glob_pattern: String,
    pub kind: WatchKind,
}

impl FileWatcher {
    pub fn new(pattern: &str) -> Self {
        Self {
            glob_pattern: pattern.to_string(),
            kind: WatchKind::all(),
        }
    }

    pub fn create_only(pattern: &str) -> Self {
        Self {
            glob_pattern: pattern.to_string(),
            kind: WatchKind::Create,
        }
    }

    pub fn change_only(pattern: &str) -> Self {
        Self {
            glob_pattern: pattern.to_string(),
            kind: WatchKind::Change,
        }
    }

    pub fn delete_only(pattern: &str) -> Self {
        Self {
            glob_pattern: pattern.to_string(),
            kind: WatchKind::Delete,
        }
    }
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self {
            folders: RwLock::new(Vec::new()),
            watchers: RwLock::new(HashMap::new()),
            document_versions: RwLock::new(HashMap::new()),
            open_documents: RwLock::new(HashMap::new()),
        }
    }

    /// Add a workspace folder
    pub fn add_folder(&self, folder: WorkspaceFolder) {
        self.folders.write().push(folder);
    }

    /// Remove a workspace folder
    pub fn remove_folder(&self, uri: &Url) {
        self.folders.write().retain(|f| &f.uri != uri);
        self.watchers.write().remove(uri.as_str());
    }

    /// Get all workspace folders
    pub fn folders(&self) -> Vec<WorkspaceFolder> {
        self.folders.read().clone()
    }

    /// Check if a file is in the workspace
    pub fn contains_file(&self, file_uri: &Url) -> bool {
        let file_path = file_uri.path();
        self.folders.read().iter().any(|folder| {
            file_path.starts_with(folder.uri.path())
        })
    }

    /// Find workspace folder for a file
    pub fn folder_for_file(&self, file_uri: &Url) -> Option<WorkspaceFolder> {
        let file_path = file_uri.path();
        self.folders.read().iter()
            .filter(|folder| file_path.starts_with(folder.uri.path()))
            .max_by_key(|folder| folder.uri.path().len())
            .cloned()
    }

    /// Register file watchers for a folder
    pub fn register_watchers(&self, folder_uri: &str, watchers: Vec<FileWatcher>) {
        self.watchers.write().insert(folder_uri.to_string(), watchers);
    }

    /// Open a document
    pub fn open_document(&self, params: DidOpenTextDocumentParams) {
        let doc = params.text_document;
        let text_doc = TextDocument::new(
            doc.uri.clone(),
            doc.language_id,
            doc.version,
            doc.text,
        );
        self.document_versions.write().insert(doc.uri.clone(), doc.version);
        self.open_documents.write().insert(doc.uri, text_doc);
    }

    /// Close a document
    pub fn close_document(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        self.document_versions.write().remove(&uri);
        self.open_documents.write().remove(&uri);
    }

    /// Update a document
    pub fn update_document(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        
        if let Some(doc) = self.open_documents.write().get_mut(&uri) {
            doc.apply_changes(params.content_changes);
            doc.version = version;
        }
        
        self.document_versions.write().insert(uri, version);
    }

    /// Get an open document
    pub fn get_document(&self, uri: &Url) -> Option<TextDocument> {
        self.open_documents.read().get(uri).cloned()
    }

    /// Get document version
    pub fn get_version(&self, uri: &Url) -> Option<i32> {
        self.document_versions.read().get(uri).copied()
    }

    /// Get all open documents
    pub fn open_documents(&self) -> Vec<TextDocument> {
        self.open_documents.read().values().cloned().collect()
    }

    /// Build workspace folders for initialization
    pub fn build_workspace_folders(&self) -> Option<Vec<WorkspaceFolder>> {
        let folders = self.folders.read();
        if folders.is_empty() {
            None
        } else {
            Some(folders.clone())
        }
    }

    /// Create DidChangeWorkspaceFoldersParams for folder changes
    pub fn create_folder_change_params(
        &self,
        added: Vec<WorkspaceFolder>,
        removed: Vec<WorkspaceFolder>,
    ) -> DidChangeWorkspaceFoldersParams {
        DidChangeWorkspaceFoldersParams {
            event: WorkspaceFoldersChangeEvent { added, removed },
        }
    }
}

impl Default for WorkspaceManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Workspace configuration
#[derive(Debug, Clone, Default)]
pub struct WorkspaceConfiguration {
    /// Settings by section
    sections: HashMap<String, serde_json::Value>,
}

impl WorkspaceConfiguration {
    pub fn new() -> Self {
        Self { sections: HashMap::new() }
    }

    /// Get a configuration value
    pub fn get<T: serde::de::DeserializeOwned>(&self, section: &str) -> Option<T> {
        self.sections.get(section)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
    }

    /// Set a configuration value
    pub fn set(&mut self, section: &str, value: serde_json::Value) {
        self.sections.insert(section.to_string(), value);
    }

    /// Get raw JSON value
    pub fn get_raw(&self, section: &str) -> Option<&serde_json::Value> {
        self.sections.get(section)
    }

    /// Build configuration items for LSP
    pub fn build_config_items(&self, items: Vec<ConfigurationItem>) -> Vec<serde_json::Value> {
        items.iter().map(|item| {
            let section = item.section.as_deref().unwrap_or("");
            self.sections.get(section).cloned().unwrap_or(serde_json::Value::Null)
        }).collect()
    }
}

/// Language-specific configuration
#[derive(Debug, Clone)]
pub struct LanguageConfiguration {
    pub language_id: String,
    pub tab_size: u32,
    pub insert_spaces: bool,
    pub trim_trailing_whitespace: bool,
    pub insert_final_newline: bool,
    pub format_on_save: bool,
    pub format_on_type: bool,
}

impl Default for LanguageConfiguration {
    fn default() -> Self {
        Self {
            language_id: "plaintext".to_string(),
            tab_size: 4,
            insert_spaces: true,
            trim_trailing_whitespace: true,
            insert_final_newline: true,
            format_on_save: false,
            format_on_type: false,
        }
    }
}

impl LanguageConfiguration {
    pub fn for_language(language_id: &str) -> Self {
        let mut config = Self::default();
        config.language_id = language_id.to_string();
        
        // Language-specific defaults
        match language_id {
            "rust" => {
                config.format_on_save = true;
            }
            "python" => {
                config.tab_size = 4;
                config.format_on_save = true;
            }
            "javascript" | "typescript" | "typescriptreact" | "javascriptreact" => {
                config.tab_size = 2;
            }
            "yaml" => {
                config.tab_size = 2;
            }
            "go" => {
                config.insert_spaces = false;
                config.format_on_save = true;
            }
            "makefile" => {
                config.insert_spaces = false;
            }
            _ => {}
        }
        
        config
    }

    pub fn to_formatting_options(&self) -> FormattingOptions {
        FormattingOptions {
            tab_size: self.tab_size,
            insert_spaces: self.insert_spaces,
            trim_trailing_whitespace: Some(self.trim_trailing_whitespace),
            insert_final_newline: Some(self.insert_final_newline),
            trim_final_newlines: Some(true),
            properties: HashMap::new(),
        }
    }
}
