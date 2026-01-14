//! Debug Adapter Protocol message types.
//!
//! Complete implementation of DAP request, response, and event messages
//! as defined by the Debug Adapter Protocol specification.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Base protocol message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProtocolMessage {
    /// Sequence number.
    pub seq: i64,
    /// Message type.
    #[serde(rename = "type")]
    pub message_type: MessageType,
}

/// Message type discriminator.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MessageType {
    Request,
    Response,
    Event,
}

/// A DAP request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    /// Sequence number.
    pub seq: i64,
    /// Command name.
    pub command: String,
    /// Arguments (command-specific).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

impl Request {
    /// Create a new request.
    pub fn new(seq: i64, command: impl Into<String>) -> Self {
        Self {
            seq,
            command: command.into(),
            arguments: None,
        }
    }

    /// Create a request with arguments.
    pub fn with_args<T: Serialize>(seq: i64, command: impl Into<String>, args: T) -> Self {
        Self {
            seq,
            command: command.into(),
            arguments: serde_json::to_value(args).ok(),
        }
    }
}

/// A DAP response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    /// Sequence number.
    pub seq: i64,
    /// Request sequence number this responds to.
    pub request_seq: i64,
    /// Whether request was successful.
    pub success: bool,
    /// Command that was requested.
    pub command: String,
    /// Error message (if not successful).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Response body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

impl Response {
    /// Create a success response.
    pub fn success(seq: i64, request_seq: i64, command: impl Into<String>) -> Self {
        Self {
            seq,
            request_seq,
            success: true,
            command: command.into(),
            message: None,
            body: None,
        }
    }

    /// Create a success response with body.
    pub fn success_with_body<T: Serialize>(
        seq: i64,
        request_seq: i64,
        command: impl Into<String>,
        body: T,
    ) -> Self {
        Self {
            seq,
            request_seq,
            success: true,
            command: command.into(),
            message: None,
            body: serde_json::to_value(body).ok(),
        }
    }

    /// Create an error response.
    pub fn error(
        seq: i64,
        request_seq: i64,
        command: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            seq,
            request_seq,
            success: false,
            command: command.into(),
            message: Some(message.into()),
            body: None,
        }
    }
}

/// A DAP event message.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// Sequence number.
    pub seq: i64,
    /// Event name.
    pub event: String,
    /// Event body.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

impl Event {
    /// Create a new event.
    pub fn new(seq: i64, event: impl Into<String>) -> Self {
        Self {
            seq,
            event: event.into(),
            body: None,
        }
    }

    /// Create an event with body.
    pub fn with_body<T: Serialize>(seq: i64, event: impl Into<String>, body: T) -> Self {
        Self {
            seq,
            event: event.into(),
            body: serde_json::to_value(body).ok(),
        }
    }
}

// ============================================================================
// Request Arguments
// ============================================================================

/// Initialize request arguments.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct InitializeRequestArguments {
    /// Client ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Client name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_name: Option<String>,
    /// Adapter ID.
    pub adapter_id: String,
    /// Locale for messages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    /// Lines start at 1.
    #[serde(default = "default_true")]
    pub lines_start_at1: bool,
    /// Columns start at 1.
    #[serde(default = "default_true")]
    pub columns_start_at1: bool,
    /// Path format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path_format: Option<PathFormat>,
    /// Support variable type.
    #[serde(default)]
    pub supports_variable_type: bool,
    /// Support variable paging.
    #[serde(default)]
    pub supports_variable_paging: bool,
    /// Support run in terminal request.
    #[serde(default)]
    pub supports_run_in_terminal_request: bool,
    /// Support memory references.
    #[serde(default)]
    pub supports_memory_references: bool,
    /// Support progress reporting.
    #[serde(default)]
    pub supports_progress_reporting: bool,
    /// Support invalidated event.
    #[serde(default)]
    pub supports_invalidated_event: bool,
    /// Support memory event.
    #[serde(default)]
    pub supports_memory_event: bool,
    /// Support ANSI styling.
    #[serde(default)]
    pub supports_ansi_styling: bool,
}

fn default_true() -> bool {
    true
}

/// Path format type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PathFormat {
    Path,
    Uri,
}

/// Launch request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchRequestArguments {
    /// Don't launch but attach.
    #[serde(default)]
    pub no_debug: bool,
    /// Restart requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub __restart: Option<Value>,
    /// All other arguments (adapter-specific).
    #[serde(flatten)]
    pub additional: HashMap<String, Value>,
}

use std::collections::HashMap;

/// Attach request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttachRequestArguments {
    /// Restart requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub __restart: Option<Value>,
    /// All other arguments (adapter-specific).
    #[serde(flatten)]
    pub additional: HashMap<String, Value>,
}

/// Set breakpoints request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetBreakpointsArguments {
    /// Source to set breakpoints in.
    pub source: Source,
    /// Breakpoints to set.
    #[serde(default)]
    pub breakpoints: Vec<SourceBreakpoint>,
    /// Whether to remove existing breakpoints.
    #[serde(default)]
    pub source_modified: bool,
}

/// Source breakpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceBreakpoint {
    /// Line number.
    pub line: i64,
    /// Column number.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// Condition expression.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
    /// Log message (logpoint).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_message: Option<String>,
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
    /// Source reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i64>,
    /// Presentation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<SourcePresentationHint>,
    /// Origin description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
}

/// Source presentation hint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SourcePresentationHint {
    Normal,
    Emphasize,
    Deemphasize,
}

/// Set function breakpoints arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetFunctionBreakpointsArguments {
    /// Breakpoints to set.
    pub breakpoints: Vec<FunctionBreakpoint>,
}

/// Function breakpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FunctionBreakpoint {
    /// Function name.
    pub name: String,
    /// Condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
    /// Hit condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hit_condition: Option<String>,
}

/// Set exception breakpoints arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SetExceptionBreakpointsArguments {
    /// Exception filters to enable.
    pub filters: Vec<String>,
    /// Filter options.
    #[serde(default)]
    pub filter_options: Vec<ExceptionFilterOptions>,
    /// Exception options.
    #[serde(default)]
    pub exception_options: Vec<ExceptionOptions>,
}

/// Exception filter options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionFilterOptions {
    /// Filter ID.
    pub filter_id: String,
    /// Condition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition: Option<String>,
}

/// Exception options.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionOptions {
    /// Exception path.
    #[serde(default)]
    pub path: Vec<ExceptionPathSegment>,
    /// Break mode.
    pub break_mode: ExceptionBreakMode,
}

/// Exception path segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionPathSegment {
    /// Negate condition.
    #[serde(default)]
    pub negate: bool,
    /// Exception names.
    pub names: Vec<String>,
}

/// Exception break mode.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ExceptionBreakMode {
    Never,
    Always,
    Unhandled,
    UserUnhandled,
}

/// Continue request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinueArguments {
    /// Thread to continue.
    pub thread_id: i64,
    /// Continue in single thread mode.
    #[serde(default)]
    pub single_thread: bool,
}

/// Next (step over) request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NextArguments {
    /// Thread to step.
    pub thread_id: i64,
    /// Single thread mode.
    #[serde(default)]
    pub single_thread: bool,
    /// Granularity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<SteppingGranularity>,
}

/// Step in request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepInArguments {
    /// Thread to step.
    pub thread_id: i64,
    /// Single thread mode.
    #[serde(default)]
    pub single_thread: bool,
    /// Target ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_id: Option<i64>,
    /// Granularity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<SteppingGranularity>,
}

/// Step out request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StepOutArguments {
    /// Thread to step.
    pub thread_id: i64,
    /// Single thread mode.
    #[serde(default)]
    pub single_thread: bool,
    /// Granularity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub granularity: Option<SteppingGranularity>,
}

/// Stepping granularity.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SteppingGranularity {
    Statement,
    Line,
    Instruction,
}

/// Stack trace request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackTraceArguments {
    /// Thread to get stack from.
    pub thread_id: i64,
    /// Start frame index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_frame: Option<i64>,
    /// Number of frames to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub levels: Option<i64>,
    /// Format options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<StackFrameFormat>,
}

/// Stack frame format.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct StackFrameFormat {
    /// Show parameters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<bool>,
    /// Show parameter types.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_types: Option<bool>,
    /// Show parameter names.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_names: Option<bool>,
    /// Show parameter values.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameter_values: Option<bool>,
    /// Show line numbers.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<bool>,
    /// Show module.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module: Option<bool>,
    /// Include all stack frames.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_all: Option<bool>,
}

/// Scopes request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopesArguments {
    /// Frame to get scopes for.
    pub frame_id: i64,
}

/// Variables request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablesArguments {
    /// Variables reference.
    pub variables_reference: i64,
    /// Filter type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<VariablesFilter>,
    /// Start index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<i64>,
    /// Count to return.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<i64>,
    /// Format options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ValueFormat>,
}

/// Variables filter.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum VariablesFilter {
    Indexed,
    Named,
}

/// Value format.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ValueFormat {
    /// Hex format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hex: Option<bool>,
}

/// Evaluate request arguments.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EvaluateArguments {
    /// Expression to evaluate.
    pub expression: String,
    /// Frame context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frame_id: Option<i64>,
    /// Context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<EvaluateContext>,
    /// Format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<ValueFormat>,
}

/// Evaluate context.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EvaluateContext {
    Watch,
    Repl,
    Hover,
    Clipboard,
    Variables,
}

// ============================================================================
// Response Bodies
// ============================================================================

/// Capabilities returned by initialize.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    /// Supports configuration done request.
    #[serde(default)]
    pub supports_configuration_done_request: bool,
    /// Supports function breakpoints.
    #[serde(default)]
    pub supports_function_breakpoints: bool,
    /// Supports conditional breakpoints.
    #[serde(default)]
    pub supports_conditional_breakpoints: bool,
    /// Supports hit conditional breakpoints.
    #[serde(default)]
    pub supports_hit_conditional_breakpoints: bool,
    /// Supports evaluate for hovers.
    #[serde(default)]
    pub supports_evaluate_for_hovers: bool,
    /// Exception breakpoint filters.
    #[serde(default)]
    pub exception_breakpoint_filters: Vec<ExceptionBreakpointsFilter>,
    /// Supports step back.
    #[serde(default)]
    pub supports_step_back: bool,
    /// Supports set variable.
    #[serde(default)]
    pub supports_set_variable: bool,
    /// Supports restart frame.
    #[serde(default)]
    pub supports_restart_frame: bool,
    /// Supports goto targets.
    #[serde(default)]
    pub supports_goto_targets_request: bool,
    /// Supports step in targets.
    #[serde(default)]
    pub supports_step_in_targets_request: bool,
    /// Supports completions.
    #[serde(default)]
    pub supports_completions_request: bool,
    /// Completion trigger characters.
    #[serde(default)]
    pub completion_trigger_characters: Vec<String>,
    /// Supports modules.
    #[serde(default)]
    pub supports_modules_request: bool,
    /// Additional module columns.
    #[serde(default)]
    pub additional_module_columns: Vec<ColumnDescriptor>,
    /// Supported checksum algorithms.
    #[serde(default)]
    pub supported_checksum_algorithms: Vec<String>,
    /// Supports restart.
    #[serde(default)]
    pub supports_restart_request: bool,
    /// Supports exception options.
    #[serde(default)]
    pub supports_exception_options: bool,
    /// Supports value formatting.
    #[serde(default)]
    pub supports_value_formatting_options: bool,
    /// Supports exception info.
    #[serde(default)]
    pub supports_exception_info_request: bool,
    /// Supports terminate debuggee.
    #[serde(default)]
    pub support_terminate_debuggee: bool,
    /// Supports suspend debuggee.
    #[serde(default)]
    pub support_suspend_debuggee: bool,
    /// Supports delayed stack trace loading.
    #[serde(default)]
    pub supports_delayed_stack_trace_loading: bool,
    /// Supports loaded sources.
    #[serde(default)]
    pub supports_loaded_sources_request: bool,
    /// Supports log points.
    #[serde(default)]
    pub supports_log_points: bool,
    /// Supports terminate threads.
    #[serde(default)]
    pub supports_terminate_threads_request: bool,
    /// Supports set expression.
    #[serde(default)]
    pub supports_set_expression: bool,
    /// Supports terminate.
    #[serde(default)]
    pub supports_terminate_request: bool,
    /// Supports data breakpoints.
    #[serde(default)]
    pub supports_data_breakpoints: bool,
    /// Supports read memory.
    #[serde(default)]
    pub supports_read_memory_request: bool,
    /// Supports write memory.
    #[serde(default)]
    pub supports_write_memory_request: bool,
    /// Supports disassemble.
    #[serde(default)]
    pub supports_disassemble_request: bool,
    /// Supports cancel.
    #[serde(default)]
    pub supports_cancel_request: bool,
    /// Supports breakpoint locations.
    #[serde(default)]
    pub supports_breakpoint_locations_request: bool,
    /// Supports clipboard context.
    #[serde(default)]
    pub supports_clipboard_context: bool,
    /// Supports stepping granularity.
    #[serde(default)]
    pub supports_stepping_granularity: bool,
    /// Supports instruction breakpoints.
    #[serde(default)]
    pub supports_instruction_breakpoints: bool,
    /// Supports exception filter options.
    #[serde(default)]
    pub supports_exception_filter_options: bool,
    /// Supports single thread execution.
    #[serde(default)]
    pub supports_single_thread_execution_requests: bool,
}

/// Exception breakpoints filter.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExceptionBreakpointsFilter {
    /// Filter identifier.
    pub filter: String,
    /// Label.
    pub label: String,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Default enabled state.
    #[serde(default)]
    pub default: bool,
    /// Supports condition.
    #[serde(default)]
    pub supports_condition: bool,
    /// Condition description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub condition_description: Option<String>,
}

/// Column descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColumnDescriptor {
    /// Attribute name.
    pub attribute_name: String,
    /// Label.
    pub label: String,
    /// Format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// Column type.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<ColumnType>,
    /// Width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<i64>,
}

/// Column type.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ColumnType {
    String,
    Number,
    Boolean,
    UnixTimestampUtc,
}

/// Stack frame.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    /// Frame ID.
    pub id: i64,
    /// Frame name.
    pub name: String,
    /// Source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line number.
    pub line: i64,
    /// Column number.
    pub column: i64,
    /// End line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,
    /// End column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i64>,
    /// Can restart.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub can_restart: Option<bool>,
    /// Instruction pointer reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_pointer_reference: Option<String>,
    /// Module ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub module_id: Option<Value>,
    /// Presentation hint.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<StackFramePresentationHint>,
}

/// Stack frame presentation hint.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StackFramePresentationHint {
    Normal,
    Label,
    Subtle,
}

/// Breakpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Breakpoint {
    /// Breakpoint ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    /// Verified.
    pub verified: bool,
    /// Message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// Source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// End line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i64>,
    /// End column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i64>,
    /// Instruction reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instruction_reference: Option<String>,
    /// Offset.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
}

/// Thread.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Thread {
    /// Thread ID.
    pub id: i64,
    /// Thread name.
    pub name: String,
}

// ============================================================================
// Event Bodies
// ============================================================================

/// Stopped event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEventBody {
    /// Reason for stopping.
    pub reason: StopReason,
    /// Description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Thread that stopped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thread_id: Option<i64>,
    /// Preserve focus hint.
    #[serde(default)]
    pub preserve_focus_hint: bool,
    /// Text for UI.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// All threads stopped.
    #[serde(default)]
    pub all_threads_stopped: bool,
    /// Hit breakpoint IDs.
    #[serde(default)]
    pub hit_breakpoint_ids: Vec<i64>,
}

/// Stop reason.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StopReason {
    Step,
    Breakpoint,
    Exception,
    Pause,
    Entry,
    Goto,
    #[serde(rename = "function breakpoint")]
    FunctionBreakpoint,
    #[serde(rename = "data breakpoint")]
    DataBreakpoint,
    #[serde(rename = "instruction breakpoint")]
    InstructionBreakpoint,
    #[serde(other)]
    Other,
}

/// Continued event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContinuedEventBody {
    /// Thread that continued.
    pub thread_id: i64,
    /// All threads continued.
    #[serde(default)]
    pub all_threads_continued: bool,
}

/// Exited event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExitedEventBody {
    /// Exit code.
    pub exit_code: i64,
}

/// Terminated event body.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TerminatedEventBody {
    /// Restart data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub restart: Option<Value>,
}

/// Thread event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadEventBody {
    /// Reason.
    pub reason: ThreadReason,
    /// Thread ID.
    pub thread_id: i64,
}

/// Thread event reason.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ThreadReason {
    Started,
    Exited,
}

/// Output event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEventBody {
    /// Output category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<OutputCategory>,
    /// Output text.
    pub output: String,
    /// Output group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<OutputGroup>,
    /// Variables reference.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables_reference: Option<i64>,
    /// Source.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    /// Line.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<i64>,
    /// Column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<i64>,
    /// Data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Output category.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputCategory {
    Console,
    Important,
    Stdout,
    Stderr,
    Telemetry,
}

/// Output group.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OutputGroup {
    Start,
    StartCollapsed,
    End,
}

/// Breakpoint event body.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointEventBody {
    /// Reason.
    pub reason: BreakpointReason,
    /// Breakpoint.
    pub breakpoint: Breakpoint,
}

/// Breakpoint event reason.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum BreakpointReason {
    Changed,
    New,
    Removed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let req = Request::new(1, "initialize");
        assert_eq!(req.seq, 1);
        assert_eq!(req.command, "initialize");
        assert!(req.arguments.is_none());
    }

    #[test]
    fn test_response_success() {
        let resp = Response::success(1, 1, "initialize");
        assert!(resp.success);
        assert!(resp.message.is_none());
    }

    #[test]
    fn test_response_error() {
        let resp = Response::error(1, 1, "launch", "Failed to launch");
        assert!(!resp.success);
        assert_eq!(resp.message, Some("Failed to launch".to_string()));
    }

    #[test]
    fn test_event_creation() {
        let event = Event::with_body(1, "stopped", StoppedEventBody {
            reason: StopReason::Breakpoint,
            description: None,
            thread_id: Some(1),
            preserve_focus_hint: false,
            text: None,
            all_threads_stopped: true,
            hit_breakpoint_ids: vec![1],
        });
        
        assert_eq!(event.event, "stopped");
        assert!(event.body.is_some());
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert!(!caps.supports_configuration_done_request);
        assert!(!caps.supports_function_breakpoints);
    }

    #[test]
    fn test_stop_reason_serialize() {
        let reason = StopReason::Breakpoint;
        let json = serde_json::to_string(&reason).unwrap();
        assert_eq!(json, "\"breakpoint\"");
    }
}
