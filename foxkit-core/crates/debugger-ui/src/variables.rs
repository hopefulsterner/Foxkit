//! Variables view

use std::collections::HashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::{DebugView, DebugViewId};

/// Variables view
pub struct VariablesView {
    /// Variable scopes
    scopes: RwLock<Vec<Scope>>,
    /// Expanded variables
    expanded: RwLock<std::collections::HashSet<VariableReference>>,
    /// Visibility
    visible: bool,
}

impl VariablesView {
    pub fn new() -> Self {
        Self {
            scopes: RwLock::new(Vec::new()),
            expanded: RwLock::new(std::collections::HashSet::new()),
            visible: true,
        }
    }

    /// Set scopes
    pub fn set_scopes(&self, scopes: Vec<Scope>) {
        *self.scopes.write() = scopes;
    }

    /// Get scopes
    pub fn scopes(&self) -> Vec<Scope> {
        self.scopes.read().clone()
    }

    /// Toggle variable expansion
    pub fn toggle_expand(&self, var_ref: VariableReference) {
        let mut expanded = self.expanded.write();
        if expanded.contains(&var_ref) {
            expanded.remove(&var_ref);
        } else {
            expanded.insert(var_ref);
        }
    }

    /// Check if variable is expanded
    pub fn is_expanded(&self, var_ref: VariableReference) -> bool {
        self.expanded.read().contains(&var_ref)
    }

    /// Clear variables
    pub fn clear(&self) {
        self.scopes.write().clear();
        self.expanded.write().clear();
    }
}

impl Default for VariablesView {
    fn default() -> Self {
        Self::new()
    }
}

impl DebugView for VariablesView {
    fn id(&self) -> DebugViewId {
        DebugViewId::Variables
    }

    fn title(&self) -> &str {
        "Variables"
    }

    fn is_visible(&self) -> bool {
        self.visible
    }

    fn show(&mut self) {
        self.visible = true;
    }

    fn hide(&mut self) {
        self.visible = false;
    }

    fn refresh(&mut self) {
        // Refresh from DAP
    }

    fn clear(&mut self) {
        VariablesView::clear(self);
    }
}

/// Variable reference
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VariableReference(pub i64);

/// Scope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scope {
    /// Scope name
    pub name: String,
    /// Variables reference
    pub variables_reference: VariableReference,
    /// Named variables count
    pub named_variables: Option<i64>,
    /// Indexed variables count
    pub indexed_variables: Option<i64>,
    /// Is expensive (lazy load)
    pub expensive: bool,
    /// Source
    pub source: Option<String>,
    /// Line
    pub line: Option<u32>,
    /// Column
    pub column: Option<u32>,
    /// End line
    pub end_line: Option<u32>,
    /// End column
    pub end_column: Option<u32>,
}

/// Variable
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    /// Variable name
    pub name: String,
    /// Variable value
    pub value: String,
    /// Type
    pub var_type: Option<String>,
    /// Presentation hint
    pub presentation_hint: Option<VariablePresentationHint>,
    /// Evaluate name
    pub evaluate_name: Option<String>,
    /// Variables reference (for children)
    pub variables_reference: VariableReference,
    /// Named variables count
    pub named_variables: Option<i64>,
    /// Indexed variables count
    pub indexed_variables: Option<i64>,
    /// Memory reference
    pub memory_reference: Option<String>,
}

impl Variable {
    /// Has children?
    pub fn has_children(&self) -> bool {
        self.variables_reference.0 > 0
    }

    /// Get display type
    pub fn display_type(&self) -> &str {
        self.var_type.as_deref().unwrap_or("unknown")
    }
}

/// Variable presentation hint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariablePresentationHint {
    /// Kind
    pub kind: Option<VariableKind>,
    /// Attributes
    pub attributes: Option<Vec<String>>,
    /// Visibility
    pub visibility: Option<VariableVisibility>,
}

/// Variable kind
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableKind {
    Property,
    Method,
    Class,
    Data,
    Event,
    BaseClass,
    InnerClass,
    Interface,
    MostDerivedClass,
    Virtual,
    DataBreakpoint,
}

/// Variable visibility
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VariableVisibility {
    Public,
    Private,
    Protected,
    Internal,
    Final,
}
