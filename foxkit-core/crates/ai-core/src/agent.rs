//! Autonomous AI Agent
//! 
//! The agent can operate in different modes:
//! - Assist: Suggests actions, user confirms
//! - Co-pilot: Works alongside user, makes small changes automatically
//! - Autonomous: Completes complex tasks independently

use std::sync::Arc;
use anyhow::Result;
use crate::{Message, Role, Provider, Tool, ToolCall, CompletionRequest};

/// Agent operating mode
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentMode {
    /// AI suggests, user confirms each action
    Assist,
    /// AI makes small changes automatically, asks for big ones
    CoPilot,
    /// AI works independently until task is complete
    Autonomous,
}

/// Autonomous AI Agent
pub struct Agent {
    provider: Arc<dyn Provider>,
    tools: Vec<Arc<dyn Tool>>,
    mode: AgentMode,
    messages: Vec<Message>,
    max_iterations: usize,
}

impl Agent {
    pub fn new(
        provider: Arc<dyn Provider>,
        tools: Vec<Arc<dyn Tool>>,
        mode: AgentMode,
    ) -> Self {
        Self {
            provider,
            tools,
            mode,
            messages: Vec::new(),
            max_iterations: 50,
        }
    }

    /// Set maximum iterations for autonomous mode
    pub fn with_max_iterations(mut self, max: usize) -> Self {
        self.max_iterations = max;
        self
    }

    /// Add system context
    pub fn with_system(&mut self, system: impl Into<String>) {
        self.messages.insert(0, Message::system(system));
    }

    /// Run the agent with a task
    pub async fn run(&mut self, task: impl Into<String>) -> Result<AgentResult> {
        self.messages.push(Message::user(task));
        
        let mut iterations = 0;
        let mut actions_taken = Vec::new();

        loop {
            if iterations >= self.max_iterations {
                return Ok(AgentResult {
                    success: false,
                    message: "Max iterations reached".into(),
                    actions: actions_taken,
                });
            }
            iterations += 1;

            // Get AI response
            let request = CompletionRequest {
                messages: self.messages.clone(),
                tools: self.tools.iter().map(|t| t.schema()).collect(),
                ..Default::default()
            };

            let response = self.provider.complete(request).await?;
            
            // Add assistant message
            self.messages.push(Message::assistant(&response.content));

            // Check if there are tool calls
            if let Some(tool_calls) = response.tool_calls {
                for call in tool_calls {
                    // In Assist mode, we'd ask for confirmation here
                    if self.mode == AgentMode::Assist {
                        // TODO: Yield for confirmation
                    }

                    // Execute the tool
                    let result = self.execute_tool(&call).await?;
                    
                    actions_taken.push(AgentAction {
                        tool: call.name.clone(),
                        arguments: call.arguments.to_string(),
                        result: result.clone(),
                    });

                    // Add tool result to conversation
                    self.messages.push(Message::tool(&call.id, &result));
                }
            } else {
                // No tool calls = task complete
                return Ok(AgentResult {
                    success: true,
                    message: response.content,
                    actions: actions_taken,
                });
            }
        }
    }

    async fn execute_tool(&self, call: &ToolCall) -> Result<String> {
        let tool = self.tools
            .iter()
            .find(|t| t.name() == call.name)
            .ok_or_else(|| anyhow::anyhow!("Tool not found: {}", call.name))?;

        let result = tool.execute(call.arguments.clone()).await?;
        Ok(result.content)
    }
}

/// Result of agent execution
#[derive(Debug, Clone)]
pub struct AgentResult {
    pub success: bool,
    pub message: String,
    pub actions: Vec<AgentAction>,
}

/// Single action taken by the agent
#[derive(Debug, Clone)]
pub struct AgentAction {
    pub tool: String,
    pub arguments: String,
    pub result: String,
}
