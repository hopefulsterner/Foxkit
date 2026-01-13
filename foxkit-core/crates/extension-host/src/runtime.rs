//! WASM-based extension runtime

use std::sync::Arc;
use anyhow::Result;
use wasmtime::{Engine, Module, Store, Linker, Instance, Memory, TypedFunc};
use tokio::sync::RwLock;

use crate::{Extension, Permission};
use crate::sandbox::Sandbox;

/// Extension runtime - executes WASM modules
pub struct ExtensionRuntime {
    /// Wasmtime engine
    engine: Engine,
    /// Compiled module
    module: Module,
    /// Store with extension state
    store: RwLock<Store<ExtensionState>>,
    /// Instance
    instance: RwLock<Option<Instance>>,
    /// Extension reference
    extension: Arc<Extension>,
}

/// Extension state (host functions can access this)
pub struct ExtensionState {
    /// Extension ID
    extension_id: String,
    /// Sandbox
    sandbox: Sandbox,
    /// Output buffer
    output: Vec<String>,
}

impl ExtensionRuntime {
    /// Create new runtime for extension
    pub async fn new(extension: &Arc<Extension>) -> Result<Self> {
        let engine = Engine::default();
        
        // Load WASM module
        let wasm_path = extension.manifest.main.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No main entry point"))?;
        
        let wasm_bytes = crate::loader::load_wasm_module(&extension.path, wasm_path).await?;
        let module = Module::new(&engine, &wasm_bytes)?;
        
        // Create store with state
        let state = ExtensionState {
            extension_id: extension.id.to_string(),
            sandbox: Sandbox::new(),
            output: Vec::new(),
        };
        let store = Store::new(&engine, state);
        
        Ok(Self {
            engine,
            module,
            store: RwLock::new(store),
            instance: RwLock::new(None),
            extension: Arc::clone(extension),
        })
    }

    /// Activate the extension
    pub async fn activate(&self) -> Result<()> {
        let mut store = self.store.write().await;
        
        // Create linker with host functions
        let linker = self.create_linker()?;
        
        // Instantiate
        let instance = linker.instantiate(&mut *store, &self.module)?;
        
        *self.instance.write().await = Some(instance.clone());
        
        // Call activate function if exists
        if let Ok(activate) = instance.get_typed_func::<(), ()>(&mut *store, "activate") {
            activate.call(&mut *store, ())?;
        }
        
        Ok(())
    }

    /// Deactivate the extension
    pub async fn deactivate(&self) -> Result<()> {
        let mut store = self.store.write().await;
        
        if let Some(instance) = self.instance.read().await.as_ref() {
            // Call deactivate function if exists
            if let Ok(deactivate) = instance.get_typed_func::<(), ()>(&mut *store, "deactivate") {
                deactivate.call(&mut *store, ())?;
            }
        }
        
        *self.instance.write().await = None;
        
        Ok(())
    }

    /// Execute a command
    pub async fn execute_command(&self, command: &str, args: &str) -> Result<String> {
        let mut store = self.store.write().await;
        
        let instance = self.instance.read().await;
        let instance = instance.as_ref()
            .ok_or_else(|| anyhow::anyhow!("Extension not activated"))?;
        
        // Get the command handler
        let handler_name = format!("command_{}", command.replace('.', "_"));
        
        // This is simplified - real implementation would use proper memory management
        if let Ok(handler) = instance.get_typed_func::<(i32, i32), i32>(&mut *store, &handler_name) {
            // TODO: Proper string passing to WASM
            let result = handler.call(&mut *store, (0, args.len() as i32))?;
            Ok(result.to_string())
        } else {
            anyhow::bail!("Command handler not found: {}", command)
        }
    }

    fn create_linker(&self) -> Result<Linker<ExtensionState>> {
        let mut linker = Linker::new(&self.engine);
        
        // Host function: log
        linker.func_wrap("foxkit", "log", |mut caller: wasmtime::Caller<'_, ExtensionState>, ptr: i32, len: i32| {
            // TODO: Read string from WASM memory
            caller.data_mut().output.push(format!("log: ptr={}, len={}", ptr, len));
        })?;
        
        // Host function: read_file (if permitted)
        linker.func_wrap("foxkit", "read_file", |caller: wasmtime::Caller<'_, ExtensionState>, _path_ptr: i32, _path_len: i32| -> i32 {
            // Check permission
            // TODO: Implement file reading
            -1
        })?;
        
        // Host function: write_file (if permitted)
        linker.func_wrap("foxkit", "write_file", |caller: wasmtime::Caller<'_, ExtensionState>, _path_ptr: i32, _path_len: i32, _data_ptr: i32, _data_len: i32| -> i32 {
            // Check permission
            // TODO: Implement file writing
            -1
        })?;
        
        // Host function: get_config
        linker.func_wrap("foxkit", "get_config", |_caller: wasmtime::Caller<'_, ExtensionState>, _key_ptr: i32, _key_len: i32| -> i32 {
            // TODO: Implement config retrieval
            -1
        })?;
        
        // Host function: show_message
        linker.func_wrap("foxkit", "show_message", |_caller: wasmtime::Caller<'_, ExtensionState>, _msg_ptr: i32, _msg_len: i32, _level: i32| {
            // TODO: Implement message display
        })?;
        
        // Host function: register_command
        linker.func_wrap("foxkit", "register_command", |_caller: wasmtime::Caller<'_, ExtensionState>, _cmd_ptr: i32, _cmd_len: i32, _handler_ptr: i32| -> i32 {
            // TODO: Implement command registration
            0
        })?;
        
        Ok(linker)
    }

    /// Get output from extension
    pub async fn drain_output(&self) -> Vec<String> {
        let mut store = self.store.write().await;
        std::mem::take(&mut store.data_mut().output)
    }
}
