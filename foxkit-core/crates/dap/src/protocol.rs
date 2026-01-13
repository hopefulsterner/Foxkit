//! DAP protocol types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// DAP message
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Message {
    Request(Request),
    Response(Response),
    Event(Event),
}

/// Request message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub seq: i64,
    pub command: String,
    #[serde(default)]
    pub arguments: Option<serde_json::Value>,
}

/// Response message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub seq: i64,
    pub request_seq: i64,
    pub success: bool,
    pub command: String,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// Event message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub seq: i64,
    pub event: String,
    #[serde(default)]
    pub body: Option<serde_json::Value>,
}

/// Initialize arguments
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InitializeArguments {
    #[serde(default)]
    pub client_id: Option<String>,
    #[serde(default)]
    pub client_name: Option<String>,
    pub adapter_id: String,
    #[serde(default)]
    pub locale: Option<String>,
    #[serde(default)]
    pub lines_start_at1: bool,
    #[serde(default)]
    pub columns_start_at1: bool,
    #[serde(default)]
    pub path_format: Option<String>,
    #[serde(default)]
    pub supports_variable_type: bool,
    #[serde(default)]
    pub supports_variable_paging: bool,
    #[serde(default)]
    pub supports_run_in_terminal_request: bool,
    #[serde(default)]
    pub supports_memory_references: bool,
    #[serde(default)]
    pub supports_progress_reporting: bool,
    #[serde(default)]
    pub supports_invalidated_event: bool,
}

/// Adapter capabilities
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    #[serde(default)]
    pub supports_configuration_done_request: bool,
    #[serde(default)]
    pub supports_function_breakpoints: bool,
    #[serde(default)]
    pub supports_conditional_breakpoints: bool,
    #[serde(default)]
    pub supports_hit_conditional_breakpoints: bool,
    #[serde(default)]
    pub supports_evaluate_for_hovers: bool,
    #[serde(default)]
    pub supports_step_back: bool,
    #[serde(default)]
    pub supports_set_variable: bool,
    #[serde(default)]
    pub supports_restart_frame: bool,
    #[serde(default)]
    pub supports_goto_targets_request: bool,
    #[serde(default)]
    pub supports_step_in_targets_request: bool,
    #[serde(default)]
    pub supports_completions_request: bool,
    #[serde(default)]
    pub supports_modules_request: bool,
    #[serde(default)]
    pub supports_restart_request: bool,
    #[serde(default)]
    pub supports_exception_options: bool,
    #[serde(default)]
    pub supports_value_formatting_options: bool,
    #[serde(default)]
    pub supports_exception_info_request: bool,
    #[serde(default)]
    pub supports_terminate_debuggee: bool,
    #[serde(default)]
    pub supports_delayed_stack_trace_loading: bool,
    #[serde(default)]
    pub supports_loaded_sources_request: bool,
    #[serde(default)]
    pub supports_log_points: bool,
    #[serde(default)]
    pub supports_terminate_threads_request: bool,
    #[serde(default)]
    pub supports_set_expression: bool,
    #[serde(default)]
    pub supports_terminate_request: bool,
    #[serde(default)]
    pub supports_data_breakpoints: bool,
    #[serde(default)]
    pub supports_read_memory_request: bool,
    #[serde(default)]
    pub supports_disassemble_request: bool,
    #[serde(default)]
    pub supports_cancel_request: bool,
    #[serde(default)]
    pub supports_breakpoint_locations_request: bool,
    #[serde(default)]
    pub supports_clipboard_context: bool,
    #[serde(default)]
    pub supports_stepping_granularity: bool,
    #[serde(default)]
    pub supports_instruction_breakpoints: bool,
    #[serde(default)]
    pub supports_exception_filter_options: bool,
}

/// Stopped event body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoppedEventBody {
    pub reason: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub thread_id: Option<i64>,
    #[serde(default)]
    pub preserve_focus_hint: bool,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub all_threads_stopped: bool,
}

/// Output event body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputEventBody {
    #[serde(default)]
    pub category: Option<String>,
    pub output: String,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub variables_reference: Option<i64>,
    #[serde(default)]
    pub source: Option<crate::Source>,
    #[serde(default)]
    pub line: Option<i64>,
    #[serde(default)]
    pub column: Option<i64>,
}

/// Breakpoint event body
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BreakpointEventBody {
    pub reason: String,
    pub breakpoint: crate::Breakpoint,
}
