//! WASM runtime using Wasmtime

use std::sync::Arc;
use parking_lot::RwLock;
use wasmtime::*;

use crate::{PluginManifest, HostFunctions, HostContext};

/// WASM runtime
pub struct WasmRuntime {
    /// Wasmtime engine
    engine: Engine,
    /// Linker with host functions
    linker: RwLock<Linker<HostContext>>,
}

impl WasmRuntime {
    pub fn new() -> anyhow::Result<Self> {
        let mut config = Config::new();
        config.async_support(true);
        config.wasm_component_model(true);
        config.consume_fuel(true); // Enable fuel-based execution limits
        
        let engine = Engine::new(&config)?;
        let linker = Linker::new(&engine);

        let runtime = Self {
            engine,
            linker: RwLock::new(linker),
        };

        // Register host functions
        runtime.register_host_functions()?;

        Ok(runtime)
    }

    /// Register host functions available to plugins
    fn register_host_functions(&self) -> anyhow::Result<()> {
        let mut linker = self.linker.write();

        // Log function
        linker.func_wrap("env", "host_log", |caller: Caller<'_, HostContext>, ptr: i32, len: i32| {
            // Read string from WASM memory
            let mem = caller.get_export("memory")
                .and_then(|e| e.into_memory());
            
            if let Some(mem) = mem {
                let data = mem.data(&caller);
                if let Ok(msg) = std::str::from_utf8(&data[ptr as usize..(ptr + len) as usize]) {
                    tracing::info!(target: "wasm_plugin", "{}", msg);
                }
            }
        })?;

        // Read file function
        linker.func_wrap_async("env", "host_read_file", |_caller: Caller<'_, HostContext>, _path_ptr: i32, _path_len: i32| {
            Box::new(async move {
                // Would read file and return handle
                0i32
            })
        })?;

        // Write file function
        linker.func_wrap_async("env", "host_write_file", |_caller: Caller<'_, HostContext>, _path_ptr: i32, _path_len: i32, _data_ptr: i32, _data_len: i32| {
            Box::new(async move {
                // Would write file
                0i32
            })
        })?;

        Ok(())
    }

    /// Instantiate a plugin
    pub async fn instantiate(
        &self,
        wasm_bytes: &[u8],
        manifest: &PluginManifest,
    ) -> anyhow::Result<PluginInstance> {
        let module = Module::new(&self.engine, wasm_bytes)?;
        
        let context = HostContext::new(manifest.clone());
        let mut store = Store::new(&self.engine, context);
        
        // Set execution limits
        store.set_fuel(1_000_000)?; // Limit computation
        
        let instance = self.linker.read()
            .instantiate_async(&mut store, &module)
            .await?;

        Ok(PluginInstance {
            store: RwLock::new(store),
            instance,
        })
    }
}

/// A plugin instance
pub struct PluginInstance {
    store: RwLock<Store<HostContext>>,
    instance: Instance,
}

impl PluginInstance {
    /// Call the activate function
    pub async fn call_activate(&self) -> anyhow::Result<()> {
        self.call_void("activate").await
    }

    /// Call the deactivate function
    pub async fn call_deactivate(&self) -> anyhow::Result<()> {
        self.call_void("deactivate").await
    }

    /// Call a void function
    async fn call_void(&self, name: &str) -> anyhow::Result<()> {
        let func = self.instance.get_func(&mut *self.store.write(), name);
        
        if let Some(func) = func {
            let typed = func.typed::<(), ()>(&self.store.read())?;
            typed.call_async(&mut *self.store.write(), ()).await?;
        }

        Ok(())
    }

    /// Call a function with serialized args
    pub async fn call_function<T: serde::de::DeserializeOwned>(
        &self,
        name: &str,
        args: &impl serde::Serialize,
    ) -> anyhow::Result<T> {
        let args_bytes = bincode::serialize(args)?;
        
        // Allocate memory in WASM for args
        let alloc = self.instance.get_typed_func::<i32, i32>(&mut *self.store.write(), "alloc")?;
        let ptr = alloc.call_async(&mut *self.store.write(), args_bytes.len() as i32).await?;
        
        // Write args to WASM memory
        let memory = self.instance.get_memory(&mut *self.store.write(), "memory")
            .ok_or_else(|| anyhow::anyhow!("No memory export"))?;
        
        memory.write(&mut *self.store.write(), ptr as usize, &args_bytes)?;

        // Call the function
        let func = self.instance.get_typed_func::<(i32, i32), i32>(&mut *self.store.write(), name)?;
        let result_ptr = func.call_async(&mut *self.store.write(), (ptr, args_bytes.len() as i32)).await?;

        // Read result length (first 4 bytes at result_ptr)
        let mut len_bytes = [0u8; 4];
        memory.read(&self.store.read(), result_ptr as usize, &mut len_bytes)?;
        let result_len = u32::from_le_bytes(len_bytes) as usize;

        // Read result bytes
        let mut result_bytes = vec![0u8; result_len];
        memory.read(&self.store.read(), (result_ptr + 4) as usize, &mut result_bytes)?;

        // Deserialize result
        let result: T = bincode::deserialize(&result_bytes)?;
        Ok(result)
    }

    /// Get remaining fuel
    pub fn remaining_fuel(&self) -> u64 {
        self.store.read().get_fuel().unwrap_or(0)
    }

    /// Add more fuel
    pub fn add_fuel(&self, fuel: u64) -> anyhow::Result<()> {
        self.store.write().set_fuel(fuel)?;
        Ok(())
    }
}
