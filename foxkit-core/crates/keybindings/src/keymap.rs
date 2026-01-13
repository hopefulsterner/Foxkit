//! Keymap definitions

use std::collections::HashMap;
use crate::{Keybinding, ResolvedBinding, Context};

/// A keymap (set of keybindings)
#[derive(Debug, Clone)]
pub struct Keymap {
    name: String,
    bindings: Vec<Keybinding>,
    /// Index for fast lookup
    index: HashMap<String, Vec<usize>>,
}

impl Keymap {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            bindings: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Add a binding
    pub fn bind(&mut self, binding: Keybinding) {
        let key = binding.key.clone();
        let idx = self.bindings.len();
        self.bindings.push(binding);
        self.index.entry(key).or_default().push(idx);
    }

    /// Add multiple bindings
    pub fn bindings(mut self, bindings: Vec<Keybinding>) -> Self {
        for binding in bindings {
            self.bind(binding);
        }
        self
    }

    /// Get binding for key and context
    pub fn get(&self, key: &str, context: &Context) -> Option<ResolvedBinding> {
        let indices = self.index.get(key)?;
        
        // Find first matching binding
        for &idx in indices {
            let binding = &self.bindings[idx];
            if binding.matches_context(context) {
                return Some(ResolvedBinding {
                    command: binding.command.clone(),
                    args: binding.args.clone(),
                });
            }
        }
        
        None
    }

    /// Check if key is a prefix for chord bindings
    pub fn has_prefix(&self, prefix: &str) -> bool {
        for key in self.index.keys() {
            if key.starts_with(prefix) && key.len() > prefix.len() && key[prefix.len()..].starts_with(' ') {
                return true;
            }
        }
        false
    }

    /// Get all bindings for a command
    pub fn bindings_for(&self, command: &str) -> Vec<String> {
        self.bindings
            .iter()
            .filter(|b| b.command == command)
            .map(|b| b.key.clone())
            .collect()
    }
}

impl Default for Keymap {
    fn default() -> Self {
        let mut keymap = Self::new("default");
        
        // File operations
        keymap.bind(Keybinding::new("ctrl+s", "workbench.action.files.save"));
        keymap.bind(Keybinding::new("ctrl+shift+s", "workbench.action.files.saveAs"));
        keymap.bind(Keybinding::new("ctrl+o", "workbench.action.files.openFile"));
        keymap.bind(Keybinding::new("ctrl+n", "workbench.action.files.newUntitledFile"));
        keymap.bind(Keybinding::new("ctrl+w", "workbench.action.closeActiveEditor"));
        
        // Edit operations
        keymap.bind(Keybinding::new("ctrl+z", "undo"));
        keymap.bind(Keybinding::new("ctrl+shift+z", "redo"));
        keymap.bind(Keybinding::new("ctrl+y", "redo"));
        keymap.bind(Keybinding::new("ctrl+x", "editor.action.clipboardCutAction"));
        keymap.bind(Keybinding::new("ctrl+c", "editor.action.clipboardCopyAction"));
        keymap.bind(Keybinding::new("ctrl+v", "editor.action.clipboardPasteAction"));
        keymap.bind(Keybinding::new("ctrl+a", "editor.action.selectAll"));
        keymap.bind(Keybinding::new("ctrl+d", "editor.action.addSelectionToNextFindMatch"));
        
        // Find/Replace
        keymap.bind(Keybinding::new("ctrl+f", "actions.find"));
        keymap.bind(Keybinding::new("ctrl+h", "editor.action.startFindReplaceAction"));
        keymap.bind(Keybinding::new("ctrl+shift+f", "workbench.action.findInFiles"));
        keymap.bind(Keybinding::new("ctrl+shift+h", "workbench.action.replaceInFiles"));
        keymap.bind(Keybinding::new("f3", "editor.action.nextMatchFindAction"));
        keymap.bind(Keybinding::new("shift+f3", "editor.action.previousMatchFindAction"));
        
        // Navigation
        keymap.bind(Keybinding::new("ctrl+g", "workbench.action.gotoLine"));
        keymap.bind(Keybinding::new("ctrl+p", "workbench.action.quickOpen"));
        keymap.bind(Keybinding::new("ctrl+shift+p", "workbench.action.showCommands"));
        keymap.bind(Keybinding::new("ctrl+shift+o", "workbench.action.gotoSymbol"));
        keymap.bind(Keybinding::new("ctrl+t", "workbench.action.showAllSymbols"));
        keymap.bind(Keybinding::new("f12", "editor.action.revealDefinition"));
        keymap.bind(Keybinding::new("alt+f12", "editor.action.peekDefinition"));
        keymap.bind(Keybinding::new("shift+f12", "editor.action.goToReferences"));
        keymap.bind(Keybinding::new("ctrl+shift+\\", "editor.action.jumpToBracket"));
        
        // View
        keymap.bind(Keybinding::new("ctrl+b", "workbench.action.toggleSidebarVisibility"));
        keymap.bind(Keybinding::new("ctrl+j", "workbench.action.togglePanel"));
        keymap.bind(Keybinding::new("ctrl+`", "workbench.action.terminal.toggleTerminal"));
        keymap.bind(Keybinding::new("ctrl+shift+e", "workbench.view.explorer"));
        keymap.bind(Keybinding::new("ctrl+shift+g", "workbench.view.scm"));
        keymap.bind(Keybinding::new("ctrl+shift+d", "workbench.view.debug"));
        keymap.bind(Keybinding::new("ctrl+shift+x", "workbench.view.extensions"));
        
        // Editor management
        keymap.bind(Keybinding::new("ctrl+\\", "workbench.action.splitEditor"));
        keymap.bind(Keybinding::new("ctrl+1", "workbench.action.focusFirstEditorGroup"));
        keymap.bind(Keybinding::new("ctrl+2", "workbench.action.focusSecondEditorGroup"));
        keymap.bind(Keybinding::new("ctrl+3", "workbench.action.focusThirdEditorGroup"));
        keymap.bind(Keybinding::new("ctrl+tab", "workbench.action.openNextRecentlyUsedEditorInGroup"));
        keymap.bind(Keybinding::new("ctrl+shift+tab", "workbench.action.openPreviousRecentlyUsedEditorInGroup"));
        
        // Code editing
        keymap.bind(Keybinding::new("ctrl+/", "editor.action.commentLine"));
        keymap.bind(Keybinding::new("ctrl+shift+/", "editor.action.blockComment"));
        keymap.bind(Keybinding::new("ctrl+]", "editor.action.indentLines"));
        keymap.bind(Keybinding::new("ctrl+[", "editor.action.outdentLines"));
        keymap.bind(Keybinding::new("alt+up", "editor.action.moveLinesUpAction"));
        keymap.bind(Keybinding::new("alt+down", "editor.action.moveLinesDownAction"));
        keymap.bind(Keybinding::new("shift+alt+up", "editor.action.copyLinesUpAction"));
        keymap.bind(Keybinding::new("shift+alt+down", "editor.action.copyLinesDownAction"));
        keymap.bind(Keybinding::new("ctrl+shift+k", "editor.action.deleteLines"));
        keymap.bind(Keybinding::new("ctrl+enter", "editor.action.insertLineAfter"));
        keymap.bind(Keybinding::new("ctrl+shift+enter", "editor.action.insertLineBefore"));
        
        // Multi-cursor
        keymap.bind(Keybinding::new("ctrl+alt+up", "editor.action.insertCursorAbove"));
        keymap.bind(Keybinding::new("ctrl+alt+down", "editor.action.insertCursorBelow"));
        keymap.bind(Keybinding::new("ctrl+shift+l", "editor.action.selectHighlights"));
        
        // Folding
        keymap.bind(Keybinding::new("ctrl+shift+[", "editor.fold"));
        keymap.bind(Keybinding::new("ctrl+shift+]", "editor.unfold"));
        keymap.bind(Keybinding::new("ctrl+k ctrl+0", "editor.foldAll"));
        keymap.bind(Keybinding::new("ctrl+k ctrl+j", "editor.unfoldAll"));
        
        // IntelliSense
        keymap.bind(Keybinding::new("ctrl+space", "editor.action.triggerSuggest"));
        keymap.bind(Keybinding::new("ctrl+shift+space", "editor.action.triggerParameterHints"));
        keymap.bind(Keybinding::new("ctrl+.", "editor.action.quickFix"));
        keymap.bind(Keybinding::new("f2", "editor.action.rename"));
        
        // Format
        keymap.bind(Keybinding::new("shift+alt+f", "editor.action.formatDocument"));
        keymap.bind(Keybinding::new("ctrl+k ctrl+f", "editor.action.formatSelection"));
        
        // Debug
        keymap.bind(Keybinding::new("f5", "workbench.action.debug.start"));
        keymap.bind(Keybinding::new("shift+f5", "workbench.action.debug.stop"));
        keymap.bind(Keybinding::new("ctrl+shift+f5", "workbench.action.debug.restart"));
        keymap.bind(Keybinding::new("f9", "editor.debug.action.toggleBreakpoint"));
        keymap.bind(Keybinding::new("f10", "workbench.action.debug.stepOver"));
        keymap.bind(Keybinding::new("f11", "workbench.action.debug.stepInto"));
        keymap.bind(Keybinding::new("shift+f11", "workbench.action.debug.stepOut"));
        
        // Integrated terminal
        keymap.bind(Keybinding::new("ctrl+shift+`", "workbench.action.terminal.new"));
        
        keymap
    }
}

/// Vim keymap
pub fn vim_keymap() -> Keymap {
    let mut keymap = Keymap::new("vim");
    
    // Normal mode basics (context: vim.mode == 'normal')
    keymap.bind(Keybinding::new("i", "vim.insertMode").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("a", "vim.insertModeAfter").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("o", "vim.insertLineBelow").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("shift+o", "vim.insertLineAbove").with_when("vim.mode == 'normal'"));
    
    // Movement
    keymap.bind(Keybinding::new("h", "vim.left").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("j", "vim.down").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("k", "vim.up").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("l", "vim.right").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("w", "vim.wordForward").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("b", "vim.wordBackward").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("0", "vim.lineStart").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("$", "vim.lineEnd").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("g g", "vim.documentStart").with_when("vim.mode == 'normal'"));
    keymap.bind(Keybinding::new("shift+g", "vim.documentEnd").with_when("vim.mode == 'normal'"));
    
    // Escape to normal mode
    keymap.bind(Keybinding::new("escape", "vim.normalMode").with_when("vim.mode != 'normal'"));
    
    keymap
}
