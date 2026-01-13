//! View registry

use std::collections::HashMap;
use crate::{View, ViewId};

/// Registry for views
pub struct ViewRegistry {
    views: HashMap<String, Box<dyn View>>,
}

impl ViewRegistry {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
        }
    }

    /// Register a view
    pub fn register(&mut self, view: Box<dyn View>) {
        let id = view.id();
        self.views.insert(id.as_str().to_string(), view);
    }

    /// Unregister a view
    pub fn unregister(&mut self, id: &ViewId) {
        self.views.remove(id.as_str());
    }

    /// Get a view
    pub fn get(&self, id: &ViewId) -> Option<&dyn View> {
        self.views.get(id.as_str()).map(|v| v.as_ref())
    }

    /// Get a view mutably
    pub fn get_mut(&mut self, id: &ViewId) -> Option<&mut dyn View> {
        self.views.get_mut(id.as_str()).map(|v| v.as_mut())
    }

    /// Check if view exists
    pub fn contains(&self, id: &ViewId) -> bool {
        self.views.contains_key(id.as_str())
    }

    /// Get all view IDs
    pub fn ids(&self) -> Vec<ViewId> {
        self.views.keys().map(|k| ViewId::from_string(k.clone())).collect()
    }

    /// Count of registered views
    pub fn count(&self) -> usize {
        self.views.len()
    }
}

impl Default for ViewRegistry {
    fn default() -> Self {
        Self::new()
    }
}
