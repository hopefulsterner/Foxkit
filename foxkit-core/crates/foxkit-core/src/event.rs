//! Event system for cross-component communication
//! 
//! This provides a pub/sub event bus inspired by both Theia's
//! messaging system and Zed's action dispatch.

use std::any::{Any, TypeId};
use std::collections::HashMap;
use parking_lot::RwLock;
use async_trait::async_trait;

/// Base trait for all events
pub trait Event: Any + Send + Sync {
    /// Event name for debugging/logging
    fn name(&self) -> &'static str;
}

/// Event handler trait
#[async_trait]
pub trait EventHandler<E: Event>: Send + Sync {
    async fn handle(&self, event: &E);
}

/// Type-erased event handler
type BoxedHandler = Box<dyn Fn(&dyn Any) + Send + Sync>;

/// Event emitter / event bus
pub struct EventEmitter {
    handlers: RwLock<HashMap<TypeId, Vec<BoxedHandler>>>,
}

impl EventEmitter {
    pub fn new() -> Self {
        Self {
            handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Subscribe to an event type
    pub fn on<E: Event + 'static, F>(&self, handler: F)
    where
        F: Fn(&E) + Send + Sync + 'static,
    {
        let type_id = TypeId::of::<E>();
        let boxed: BoxedHandler = Box::new(move |any| {
            if let Some(event) = any.downcast_ref::<E>() {
                handler(event);
            }
        });

        self.handlers
            .write()
            .entry(type_id)
            .or_default()
            .push(boxed);
    }

    /// Emit an event to all subscribers
    pub fn emit<E: Event + 'static>(&self, event: E) {
        let type_id = TypeId::of::<E>();
        
        if let Some(handlers) = self.handlers.read().get(&type_id) {
            for handler in handlers {
                handler(&event);
            }
        }
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Core Events
// ============================================================================

/// Workspace events
pub mod workspace {
    use super::*;

    pub struct WorkspaceOpened {
        pub path: std::path::PathBuf,
    }
    
    impl Event for WorkspaceOpened {
        fn name(&self) -> &'static str { "workspace.opened" }
    }

    pub struct WorkspaceClosed;
    
    impl Event for WorkspaceClosed {
        fn name(&self) -> &'static str { "workspace.closed" }
    }
}

/// Editor events
pub mod editor {
    use super::*;

    pub struct FileOpened {
        pub path: std::path::PathBuf,
    }
    
    impl Event for FileOpened {
        fn name(&self) -> &'static str { "editor.file_opened" }
    }

    pub struct FileSaved {
        pub path: std::path::PathBuf,
    }
    
    impl Event for FileSaved {
        fn name(&self) -> &'static str { "editor.file_saved" }
    }

    pub struct BufferChanged {
        pub path: std::path::PathBuf,
    }
    
    impl Event for BufferChanged {
        fn name(&self) -> &'static str { "editor.buffer_changed" }
    }
}

/// AI events
pub mod ai {
    use super::*;

    pub struct AssistantActivated;
    
    impl Event for AssistantActivated {
        fn name(&self) -> &'static str { "ai.assistant_activated" }
    }

    pub struct CompletionRequested {
        pub context: String,
    }
    
    impl Event for CompletionRequested {
        fn name(&self) -> &'static str { "ai.completion_requested" }
    }
}

/// Collaboration events
pub mod collab {
    use super::*;

    pub struct PeerJoined {
        pub peer_id: String,
        pub name: String,
    }
    
    impl Event for PeerJoined {
        fn name(&self) -> &'static str { "collab.peer_joined" }
    }

    pub struct PeerLeft {
        pub peer_id: String,
    }
    
    impl Event for PeerLeft {
        fn name(&self) -> &'static str { "collab.peer_left" }
    }
}
