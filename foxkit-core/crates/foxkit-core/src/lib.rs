//! # Foxkit Core
//! 
//! The foundational layer of Foxkit - provides core abstractions,
//! dependency injection, messaging, and lifecycle management.

pub mod app;
pub mod context;
pub mod event;
pub mod registry;
pub mod settings;

use std::sync::Arc;
use parking_lot::RwLock;
use anyhow::Result;

pub use app::App;
pub use context::Context;
pub use event::{Event, EventEmitter, EventHandler};
pub use registry::ServiceRegistry;
pub use settings::Settings;

/// The main Foxkit application instance
pub struct Foxkit {
    /// Global application context
    context: Arc<Context>,
    /// Service registry for dependency injection
    services: Arc<ServiceRegistry>,
    /// Application settings
    settings: Arc<RwLock<Settings>>,
    /// Event bus for cross-component communication
    event_bus: Arc<EventEmitter>,
}

impl Foxkit {
    /// Create a new Foxkit instance
    pub fn new() -> Result<Self> {
        let settings = Arc::new(RwLock::new(Settings::load_or_default()?));
        let event_bus = Arc::new(EventEmitter::new());
        let services = Arc::new(ServiceRegistry::new());
        let context = Arc::new(Context::new(
            Arc::clone(&settings),
            Arc::clone(&event_bus),
            Arc::clone(&services),
        ));

        Ok(Self {
            context,
            services,
            settings,
            event_bus,
        })
    }

    /// Run the main application loop
    pub fn run(self) -> Result<()> {
        tracing::info!("🦊 Foxkit v{} initialized", env!("CARGO_PKG_VERSION"));
        
        // Initialize core services
        self.init_services()?;
        
        // Start the UI (platform-specific)
        #[cfg(feature = "native")]
        {
            foxkit_gpui::run(self.context)?;
        }
        
        #[cfg(feature = "web")]
        {
            // WASM entry point
            platform_web::run(self.context)?;
        }
        
        Ok(())
    }

    fn init_services(&self) -> Result<()> {
        // Register core services
        // These will be initialized lazily via the service registry
        tracing::debug!("Registering core services...");
        
        // Workspace service
        // Editor service  
        // Terminal service
        // AI service
        // Collaboration service
        // Monorepo intelligence service
        // Extension host service
        
        Ok(())
    }
}
