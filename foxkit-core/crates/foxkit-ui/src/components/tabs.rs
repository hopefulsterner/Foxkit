//! Tab Component
//!
//! Tab bar for editor tabs and panel tabs.

use crate::components::activity_bar::Color;

/// Tab bar state
#[derive(Debug, Clone)]
pub struct TabBar {
    pub tabs: Vec<Tab>,
    pub active_index: Option<usize>,
}

impl TabBar {
    pub fn new() -> Self {
        Self {
            tabs: vec![],
            active_index: None,
        }
    }

    /// Add a tab
    pub fn add_tab(&mut self, tab: Tab) -> usize {
        let index = self.tabs.len();
        self.tabs.push(tab);
        if self.active_index.is_none() {
            self.active_index = Some(index);
        }
        index
    }

    /// Remove a tab by index
    pub fn remove_tab(&mut self, index: usize) -> Option<Tab> {
        if index < self.tabs.len() {
            let tab = self.tabs.remove(index);
            
            // Adjust active index
            if let Some(active) = self.active_index {
                if active == index {
                    // Select previous tab or next if at start
                    self.active_index = if self.tabs.is_empty() {
                        None
                    } else if index > 0 {
                        Some(index - 1)
                    } else {
                        Some(0)
                    };
                } else if active > index {
                    self.active_index = Some(active - 1);
                }
            }
            
            Some(tab)
        } else {
            None
        }
    }

    /// Set active tab
    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active_index = Some(index);
        }
    }

    /// Get active tab
    pub fn active_tab(&self) -> Option<&Tab> {
        self.active_index.and_then(|i| self.tabs.get(i))
    }

    /// Mark tab as modified
    pub fn set_modified(&mut self, index: usize, modified: bool) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.modified = modified;
        }
    }

    /// Find tab by identifier
    pub fn find_by_id(&self, id: &str) -> Option<usize> {
        self.tabs.iter().position(|t| t.id == id)
    }
}

impl Default for TabBar {
    fn default() -> Self {
        Self::new()
    }
}

/// A single tab
#[derive(Debug, Clone)]
pub struct Tab {
    pub id: String,
    pub title: String,
    pub icon: Option<&'static str>,
    pub modified: bool,
    pub pinned: bool,
    pub preview: bool, // Italic preview tab
    pub tooltip: Option<String>,
}

impl Tab {
    pub fn new(id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            icon: None,
            modified: false,
            pinned: false,
            preview: false,
            tooltip: None,
        }
    }

    pub fn with_icon(mut self, icon: &'static str) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn modified(mut self) -> Self {
        self.modified = true;
        self
    }

    pub fn pinned(mut self) -> Self {
        self.pinned = true;
        self
    }

    pub fn preview(mut self) -> Self {
        self.preview = true;
        self
    }

    pub fn with_tooltip(mut self, tooltip: impl Into<String>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }
}

/// Rendered tab bar view
#[derive(Debug, Clone)]
pub struct TabBarView {
    pub tabs: Vec<TabView>,
    pub active_index: Option<usize>,
    pub colors: TabBarColors,
}

/// Rendered tab view
#[derive(Debug, Clone)]
pub struct TabView {
    pub title: String,
    pub icon: Option<&'static str>,
    pub modified: bool,
    pub pinned: bool,
    pub preview: bool,
    pub is_active: bool,
    pub tooltip: Option<String>,
}

#[derive(Debug, Clone, Copy)]
pub struct TabBarColors {
    pub background: Color,
    pub active_bg: Color,
    pub active_fg: Color,
    pub inactive_bg: Color,
    pub inactive_fg: Color,
    pub border: Color,
    pub modified_dot: Color,
}
