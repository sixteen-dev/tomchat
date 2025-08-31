mod audio;
mod speech;
mod input;
mod config;
mod app;

use anyhow::Result;
use clap::Parser;
use tracing::{info, error};
use tracing_subscriber::{self, EnvFilter};

use crate::app::TomChatApp;
use crate::config::Config;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Enable GUI mode - outputs JSON status events to stdout
    #[arg(long)]
    gui_mode: bool,
    
    /// Enable test mode - automatically triggers recording cycle for testing
    #[arg(long)]
    test_mode: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging - in GUI mode, suppress normal logs to avoid interfering with JSON output
    if args.gui_mode {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("error"))
            .with_writer(std::io::stderr)
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(EnvFilter::new("tomchat=info,warn,error"))
            .init();
    }

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
        Ok(mut app) => {
            app.set_gui_mode(args.gui_mode);
            app.set_test_mode(args.test_mode);
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
