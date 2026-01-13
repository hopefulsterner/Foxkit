//! Debug toolbar

use parking_lot::RwLock;
use serde::{Deserialize, Serialize};

use crate::ToolbarState;

/// Debug toolbar
pub struct DebugToolbar {
    /// Current state
    state: RwLock<ToolbarState>,
    /// Enabled actions
    actions: RwLock<ToolbarActions>,
}

impl DebugToolbar {
    pub fn new() -> Self {
        Self {
            state: RwLock::new(ToolbarState::Stopped),
            actions: RwLock::new(ToolbarActions::default()),
        }
    }

    /// Get current state
    pub fn state(&self) -> ToolbarState {
        *self.state.read()
    }

    /// Set state
    pub fn set_state(&self, state: ToolbarState) {
        *self.state.write() = state;
        self.update_actions(state);
    }

    /// Update enabled actions based on state
    fn update_actions(&self, state: ToolbarState) {
        let mut actions = self.actions.write();
        
        match state {
            ToolbarState::Stopped => {
                actions.continue_enabled = false;
                actions.pause_enabled = false;
                actions.step_over_enabled = false;
                actions.step_into_enabled = false;
                actions.step_out_enabled = false;
                actions.restart_enabled = false;
                actions.stop_enabled = false;
            }
            ToolbarState::Running => {
                actions.continue_enabled = false;
                actions.pause_enabled = true;
                actions.step_over_enabled = false;
                actions.step_into_enabled = false;
                actions.step_out_enabled = false;
                actions.restart_enabled = true;
                actions.stop_enabled = true;
            }
            ToolbarState::Paused => {
                actions.continue_enabled = true;
                actions.pause_enabled = false;
                actions.step_over_enabled = true;
                actions.step_into_enabled = true;
                actions.step_out_enabled = true;
                actions.restart_enabled = true;
                actions.stop_enabled = true;
            }
        }
    }

    /// Get enabled actions
    pub fn actions(&self) -> ToolbarActions {
        *self.actions.read()
    }

    /// Is action enabled?
    pub fn is_enabled(&self, action: DebugAction) -> bool {
        let actions = self.actions.read();
        match action {
            DebugAction::Continue => actions.continue_enabled,
            DebugAction::Pause => actions.pause_enabled,
            DebugAction::StepOver => actions.step_over_enabled,
            DebugAction::StepInto => actions.step_into_enabled,
            DebugAction::StepOut => actions.step_out_enabled,
            DebugAction::Restart => actions.restart_enabled,
            DebugAction::Stop => actions.stop_enabled,
        }
    }
}

impl Default for DebugToolbar {
    fn default() -> Self {
        Self::new()
    }
}

/// Toolbar actions enabled state
#[derive(Debug, Clone, Copy, Default)]
pub struct ToolbarActions {
    pub continue_enabled: bool,
    pub pause_enabled: bool,
    pub step_over_enabled: bool,
    pub step_into_enabled: bool,
    pub step_out_enabled: bool,
    pub restart_enabled: bool,
    pub stop_enabled: bool,
}

/// Debug action
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DebugAction {
    Continue,
    Pause,
    StepOver,
    StepInto,
    StepOut,
    Restart,
    Stop,
}

impl DebugAction {
    /// Get action ID
    pub fn id(&self) -> &str {
        match self {
            Self::Continue => "debug.continue",
            Self::Pause => "debug.pause",
            Self::StepOver => "debug.stepOver",
            Self::StepInto => "debug.stepInto",
            Self::StepOut => "debug.stepOut",
            Self::Restart => "debug.restart",
            Self::Stop => "debug.stop",
        }
    }

    /// Get action label
    pub fn label(&self) -> &str {
        match self {
            Self::Continue => "Continue",
            Self::Pause => "Pause",
            Self::StepOver => "Step Over",
            Self::StepInto => "Step Into",
            Self::StepOut => "Step Out",
            Self::Restart => "Restart",
            Self::Stop => "Stop",
        }
    }

    /// Get keyboard shortcut
    pub fn shortcut(&self) -> &str {
        match self {
            Self::Continue => "F5",
            Self::Pause => "F6",
            Self::StepOver => "F10",
            Self::StepInto => "F11",
            Self::StepOut => "Shift+F11",
            Self::Restart => "Ctrl+Shift+F5",
            Self::Stop => "Shift+F5",
        }
    }

    /// Get icon name
    pub fn icon(&self) -> &str {
        match self {
            Self::Continue => "debug-continue",
            Self::Pause => "debug-pause",
            Self::StepOver => "debug-step-over",
            Self::StepInto => "debug-step-into",
            Self::StepOut => "debug-step-out",
            Self::Restart => "debug-restart",
            Self::Stop => "debug-stop",
        }
    }
}

/// Status bar item for debug status
#[derive(Debug, Clone)]
pub struct DebugStatusItem {
    /// Status text
    pub text: String,
    /// Tooltip
    pub tooltip: String,
    /// Icon
    pub icon: String,
    /// Is visible
    pub visible: bool,
}

impl DebugStatusItem {
    pub fn from_state(state: ToolbarState) -> Self {
        match state {
            ToolbarState::Stopped => Self {
                text: String::new(),
                tooltip: String::new(),
                icon: String::new(),
                visible: false,
            },
            ToolbarState::Running => Self {
                text: "Running".to_string(),
                tooltip: "Debugging in progress".to_string(),
                icon: "debug-start".to_string(),
                visible: true,
            },
            ToolbarState::Paused => Self {
                text: "Paused".to_string(),
                tooltip: "Debugger paused".to_string(),
                icon: "debug-pause".to_string(),
                visible: true,
            },
        }
    }
}
