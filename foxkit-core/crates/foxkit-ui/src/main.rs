//! Foxkit IDE - Main entry point
//!
//! Launches the Foxkit IDE with the full UI component system.

use anyhow::Result;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("foxkit=info".parse().unwrap())
                .add_directive("wgpu=warn".parse().unwrap())
        )
        .init();

    tracing::info!("🦊 Starting Foxkit IDE...");

    // Run the application using foxkit-gpui window system
    foxkit_gpui::run()?;

    Ok(())
}
