//! Debug Adapter Protocol variable inspection and evaluation.
//!
//! This module provides comprehensive support for:
//! - Variable scopes and hierarchies
//! - Expression evaluation
//! - Watch expressions
//! - Memory inspection
//! - Variable modification

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A variable in the debug session.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// Variable name.
    pub name: String,
    /// Variable value as a string.
    pub value: String,
    /// Type of the variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Reference for structured variables (to fetch children).
    /// 0 means no children.
    #[serde(default)]
    pub variables_reference: i64,
    /// Number of named children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i64>,
    /// Number of indexed children (array elements).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<i64>,
    /// Memory reference for this variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_reference: Option<String>,
    /// Presentation hint for UI rendering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<VariablePresentationHint>,
    /// Expression that evaluates to this variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluate_name: Option<String>,
}

impl Variable {
    /// Create a simple variable with name and value.
    pub fn simple(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            r#type: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
            presentation_hint: None,
            evaluate_name: None,
        }
    }

    /// Create a variable with a type.
    pub fn typed(name: impl Into<String>, value: impl Into<String>, typ: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            r#type: Some(typ.into()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
            presentation_hint: None,
            evaluate_name: None,
        }
    }

    /// Create a structured variable (with children).
    pub fn structured(
        name: impl Into<String>,
        value: impl Into<String>,
        reference: i64,
    ) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            r#type: None,
            variables_reference: reference,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
            presentation_hint: None,
            evaluate_name: None,
        }
    }

    /// Check if variable has children.
    pub fn has_children(&self) -> bool {
        self.variables_reference > 0
    }

    /// Set the type.
    pub fn with_type(mut self, typ: impl Into<String>) -> Self {
        self.r#type = Some(typ.into());
        self
    }

    /// Set named variables count.
    pub fn with_named_variables(mut self, count: i64) -> Self {
        self.named_variables = Some(count);
        self
    }

    /// Set indexed variables count.
    pub fn with_indexed_variables(mut self, count: i64) -> Self {
        self.indexed_variables = Some(count);
        self
    }

    /// Set presentation hint.
    pub fn with_hint(mut self, hint: VariablePresentationHint) -> Self {
        self.presentation_hint = Some(hint);
        self
    }

    /// Set evaluate name.
    pub fn with_evaluate_name(mut self, name: impl Into<String>) -> Self {
        self.evaluate_name = Some(name.into());
        self
    }
}

/// Presentation hint for how to render a variable.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VariablePresentationHint {
    /// Kind of variable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<VariableKind>,
    /// Additional attributes.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<VariableAttribute>,
    /// Visibility.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<VariableVisibility>,
    /// Whether value should be shown lazily.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lazy: Option<bool>,
}

/// Variable kind for presentation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VariableKind {
    /// Property of an object.
    Property,
    /// Method/function.
    Method,
    /// Class.
    Class,
    /// Data value.
    Data,
    /// Event.
    Event,
    /// Base class.
    BaseClass,
    /// Inner class.
    InnerClass,
    /// Interface.
    Interface,
    /// Virtual property.
    Virtual,
    /// Boolean value.
    Boolean,
    /// String value.
    String,
    /// Number value.
    Number,
    /// Array/list.
    Array,
    /// Map/dictionary.
    Map,
}

/// Variable attribute for presentation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VariableAttribute {
    /// Static member.
    Static,
    /// Constant.
    Constant,
    /// Read-only.
    ReadOnly,
    /// Raw string (don't escape).
    RawString,
    /// Has object ID.
    HasObjectId,
    /// Can have object ID.
    CanHaveObjectId,
    /// Has side effects when evaluated.
    HasSideEffects,
    /// Has data breakpoint.
    HasDataBreakpoint,
}

/// Variable visibility.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VariableVisibility {
    Public,
    Private,
    Protected,
    Internal,
    Final,
}

/// A scope containing variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    /// Scope name (e.g., "Locals", "Arguments", "Globals").
    pub name: String,
    /// Variables reference to fetch variables in this scope.
    pub variables_reference: i64,
    /// Number of named variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i64>,
    /// Number of indexed variables.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<i64>,
    /// Whether this scope is expensive to retrieve.
    #[serde(default)]
    pub expensive: bool,
    /// Source location where scope starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line where scope starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Column where scope starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// Line where scope ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,
    /// Column where scope ends.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i64>,
    /// Presentation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<ScopePresentationHint>,
}

impl Scope {
    /// Create a locals scope.
    pub fn locals(variables_reference: i64) -> Self {
        Self {
            name: "Locals".to_string(),
            variables_reference,
            named_variables: None,
            indexed_variables: None,
            expensive: false,
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            presentation_hint: Some(ScopePresentationHint::Locals),
        }
    }

    /// Create an arguments scope.
    pub fn arguments(variables_reference: i64) -> Self {
        Self {
            name: "Arguments".to_string(),
            variables_reference,
            named_variables: None,
            indexed_variables: None,
            expensive: false,
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            presentation_hint: Some(ScopePresentationHint::Arguments),
        }
    }

    /// Create a globals scope.
    pub fn globals(variables_reference: i64) -> Self {
        Self {
            name: "Globals".to_string(),
            variables_reference,
            named_variables: None,
            indexed_variables: None,
            expensive: true, // Globals often expensive to enumerate
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            presentation_hint: Some(ScopePresentationHint::Globals),
        }
    }

    /// Create a registers scope.
    pub fn registers(variables_reference: i64) -> Self {
        Self {
            name: "Registers".to_string(),
            variables_reference,
            named_variables: None,
            indexed_variables: None,
            expensive: false,
            source: None,
            line: None,
            column: None,
            end_line: None,
            end_column: None,
            presentation_hint: Some(ScopePresentationHint::Registers),
        }
    }
}

/// Scope presentation hint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ScopePresentationHint {
    /// Arguments scope.
    Arguments,
    /// Locals scope.
    Locals,
    /// Globals scope.
    Globals,
    /// Registers scope.
    Registers,
}

/// Source reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    /// Source name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Source path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    /// Source reference (for generated sources).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,
    /// Presentation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<SourcePresentationHint>,
    /// Origin description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    /// Sources that were used to generate this source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    /// Adapter-specific data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter_data: Option<serde_json::Value>,
    /// Checksums.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub checksums: Vec<Checksum>,
}

/// Source presentation hint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourcePresentationHint {
    Normal,
    Emphasize,
    Deemphasize,
}

/// Source checksum.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Checksum {
    /// Algorithm used.
    pub algorithm: ChecksumAlgorithm,
    /// Checksum value.
    pub checksum: String,
}

/// Checksum algorithm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChecksumAlgorithm {
    MD5,
    SHA1,
    SHA256,
    #[serde(rename = "timestamp")]
    Timestamp,
}

/// Evaluate request context.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EvaluateContext {
    /// Watch expression.
    Watch,
    /// Repl/console.
    Repl,
    /// Hover tooltip.
    Hover,
    /// Clipboard evaluation.
    Clipboard,
    /// Variable view.
    Variables,
}

/// Result of an evaluation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateResult {
    /// String representation of result.
    pub result: String,
    /// Type of result.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    /// Variables reference for structured results.
    #[serde(default)]
    pub variables_reference: i64,
    /// Number of named children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i64>,
    /// Number of indexed children.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<i64>,
    /// Memory reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_reference: Option<String>,
    /// Presentation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<VariablePresentationHint>,
}

impl EvaluateResult {
    /// Create a simple result.
    pub fn simple(result: impl Into<String>) -> Self {
        Self {
            result: result.into(),
            r#type: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
            presentation_hint: None,
        }
    }

    /// Create a typed result.
    pub fn typed(result: impl Into<String>, typ: impl Into<String>) -> Self {
        Self {
            result: result.into(),
            r#type: Some(typ.into()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            memory_reference: None,
            presentation_hint: None,
        }
    }
}

/// Watch expression manager.
pub struct WatchManager {
    /// Watch expressions.
    watches: Vec<WatchExpression>,
    /// Next watch ID.
    next_id: u64,
}

/// A watch expression.
#[derive(Debug, Clone)]
pub struct WatchExpression {
    /// Unique ID.
    pub id: u64,
    /// Expression string.
    pub expression: String,
    /// Last evaluated result.
    pub result: Option<EvaluateResult>,
    /// Whether evaluation failed.
    pub error: Option<String>,
    /// Frame ID for evaluation context.
    pub frame_id: Option<i64>,
}

impl WatchManager {
    /// Create a new watch manager.
    pub fn new() -> Self {
        Self {
            watches: Vec::new(),
            next_id: 1,
        }
    }

    /// Add a watch expression.
    pub fn add(&mut self, expression: impl Into<String>) -> u64 {
        let id = self.next_id;
        self.next_id += 1;

        self.watches.push(WatchExpression {
            id,
            expression: expression.into(),
            result: None,
            error: None,
            frame_id: None,
        });

        id
    }

    /// Remove a watch expression.
    pub fn remove(&mut self, id: u64) -> bool {
        if let Some(pos) = self.watches.iter().position(|w| w.id == id) {
            self.watches.remove(pos);
            true
        } else {
            false
        }
    }

    /// Update watch expression.
    pub fn update(&mut self, id: u64, expression: impl Into<String>) -> bool {
        if let Some(watch) = self.watches.iter_mut().find(|w| w.id == id) {
            watch.expression = expression.into();
            watch.result = None;
            watch.error = None;
            true
        } else {
            false
        }
    }

    /// Get all watches.
    pub fn watches(&self) -> &[WatchExpression] {
        &self.watches
    }

    /// Set result for a watch.
    pub fn set_result(&mut self, id: u64, result: EvaluateResult) {
        if let Some(watch) = self.watches.iter_mut().find(|w| w.id == id) {
            watch.result = Some(result);
            watch.error = None;
        }
    }

    /// Set error for a watch.
    pub fn set_error(&mut self, id: u64, error: impl Into<String>) {
        if let Some(watch) = self.watches.iter_mut().find(|w| w.id == id) {
            watch.result = None;
            watch.error = Some(error.into());
        }
    }

    /// Clear all results (e.g., when execution continues).
    pub fn clear_results(&mut self) {
        for watch in &mut self.watches {
            watch.result = None;
            watch.error = None;
        }
    }
}

impl Default for WatchManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Variable store for caching variable hierarchies.
pub struct VariableStore {
    /// Variables by reference ID.
    variables: HashMap<i64, Vec<Variable>>,
    /// Next reference ID.
    next_reference: i64,
    /// Root scope references.
    scope_references: HashMap<i64, i64>, // frame_id -> variables_reference
}

impl VariableStore {
    /// Create a new variable store.
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            next_reference: 1,
            scope_references: HashMap::new(),
        }
    }

    /// Allocate a new reference ID.
    pub fn allocate_reference(&mut self) -> i64 {
        let id = self.next_reference;
        self.next_reference += 1;
        id
    }

    /// Store variables for a reference.
    pub fn store(&mut self, reference: i64, variables: Vec<Variable>) {
        self.variables.insert(reference, variables);
    }

    /// Get variables for a reference.
    pub fn get(&self, reference: i64) -> Option<&Vec<Variable>> {
        self.variables.get(&reference)
    }

    /// Register a scope reference for a frame.
    pub fn register_scope(&mut self, frame_id: i64, variables_reference: i64) {
        self.scope_references.insert(frame_id, variables_reference);
    }

    /// Get scope reference for a frame.
    pub fn scope_reference(&self, frame_id: i64) -> Option<i64> {
        self.scope_references.get(&frame_id).copied()
    }

    /// Clear all stored variables (e.g., on continue).
    pub fn clear(&mut self) {
        self.variables.clear();
        self.scope_references.clear();
        // Don't reset next_reference to avoid ID reuse within session
    }

    /// Clear variables for specific references.
    pub fn clear_references(&mut self, references: &[i64]) {
        for reference in references {
            self.variables.remove(reference);
        }
    }
}

impl Default for VariableStore {
    fn default() -> Self {
        Self::new()
    }
}

/// Data breakpoint information.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataBreakpoint {
    /// Data breakpoint ID.
    pub data_id: String,
    /// Access type.
    pub access_type: DataBreakpointAccessType,
    /// Condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
}

/// Data breakpoint access type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum DataBreakpointAccessType {
    Read,
    Write,
    ReadWrite,
}

/// Information about data breakpoint capability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataBreakpointInfo {
    /// Data breakpoint ID (null if not available).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    /// Description.
    pub description: String,
    /// Supported access types.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub access_types: Vec<DataBreakpointAccessType>,
    /// Whether data breakpoint can be set.
    #[serde(default)]
    pub can_persist: bool,
}

/// Memory read result.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoryContents {
    /// Start address.
    pub address: String,
    /// Address where unreadable memory starts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unreadable_bytes: Option<i64>,
    /// Base64-encoded memory contents.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_creation() {
        let var = Variable::typed("x", "42", "int")
            .with_hint(VariablePresentationHint {
                kind: Some(VariableKind::Data),
                ..Default::default()
            });
        
        assert_eq!(var.name, "x");
        assert_eq!(var.value, "42");
        assert_eq!(var.r#type, Some("int".to_string()));
        assert!(!var.has_children());
    }

    #[test]
    fn test_structured_variable() {
        let var = Variable::structured("obj", "Object {...}", 123)
            .with_named_variables(5);
        
        assert!(var.has_children());
        assert_eq!(var.variables_reference, 123);
    }

    #[test]
    fn test_scope_creation() {
        let locals = Scope::locals(1);
        let args = Scope::arguments(2);
        let globals = Scope::globals(3);

        assert_eq!(locals.name, "Locals");
        assert!(!locals.expensive);
        assert!(globals.expensive);
        assert_eq!(args.presentation_hint, Some(ScopePresentationHint::Arguments));
    }

    #[test]
    fn test_watch_manager() {
        let mut manager = WatchManager::new();
        
        let id1 = manager.add("x + y");
        let id2 = manager.add("obj.value");
        
        assert_eq!(manager.watches().len(), 2);
        
        manager.set_result(id1, EvaluateResult::simple("10"));
        assert!(manager.watches()[0].result.is_some());
        
        manager.remove(id2);
        assert_eq!(manager.watches().len(), 1);
    }

    #[test]
    fn test_variable_store() {
        let mut store = VariableStore::new();
        
        let ref1 = store.allocate_reference();
        let ref2 = store.allocate_reference();
        
        assert_ne!(ref1, ref2);
        
        store.store(ref1, vec![
            Variable::simple("a", "1"),
            Variable::simple("b", "2"),
        ]);
        
        assert_eq!(store.get(ref1).unwrap().len(), 2);
        assert!(store.get(ref2).is_none());
    }
}
