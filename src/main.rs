mod audio;
mod speech;
mod input;
mod config;
mod app;

use anyhow::Result;
use tracing::{info, error};
use tracing_subscriber::{self, EnvFilter};

use crate::app::TomChatApp;
use crate::config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new("tomchat=info,warn,error"))
        .init();

    // Print banner
    info!("🐕 TomChat - Speech-to-Text Hotkey Application");
    info!("   Named after Tommy");
    info!("   Powered by Rust + Professional Crates");
    info!("   =====================================");

    // Load configuration
    let config = match Config::load() {
        Ok(config) => {
            info!("✅ Configuration loaded successfully");
            config
        }
        Err(e) => {
            error!("❌ Failed to load configuration: {}", e);
            error!("   Make sure config.toml exists in the current directory");
            return Err(e);
        }
    };

    // Initialize and run the application
    match TomChatApp::new(config).await {
        Ok(app) => {
            info!("🚀 Starting TomChat...");
            
            // Set up graceful shutdown
            let ctrl_c = tokio::signal::ctrl_c();
            
            tokio::select! {
                result = app.run() => {
                    match result {
                        Ok(_) => info!("✅ TomChat finished successfully"),
                        Err(e) => error!("❌ TomChat error: {}", e),
                    }
                }
                _ = ctrl_c => {
                    info!("🛑 Ctrl+C received, shutting down...");
                }
            }
        }
        Err(e) => {
            error!("❌ Failed to initialize TomChat: {}", e);
            return Err(e);
        }
    }

    info!("👋 TomChat goodbye!");
    Ok(())
}
