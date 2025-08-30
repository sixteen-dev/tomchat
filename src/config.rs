use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub hotkey: HotkeyConfig,
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub whisper: WhisperConfig,
    pub text: TextConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HotkeyConfig {
    pub combination: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioConfig {
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_duration_ms: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VadConfig {
    pub sensitivity: VadSensitivity,
    pub timeout_ms: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum VadSensitivity {
    Low,
    Normal,
    High,
    VeryHigh,
}

impl VadSensitivity {
    pub fn to_webrtc_mode(&self) -> i32 {
        match self {
            VadSensitivity::Low => 0,
            VadSensitivity::Normal => 1,
            VadSensitivity::High => 2,
            VadSensitivity::VeryHigh => 3,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WhisperConfig {
    pub model_path: PathBuf,
    pub language: String,
    pub translate: bool,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextConfig {
    pub typing_delay_ms: u64,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = std::env::current_dir()?.join("config.toml");
        let config_str = std::fs::read_to_string(config_path)?;
        let mut config: Config = toml::from_str(&config_str)?;
        
        // Override with environment variables if set
        if let Ok(model_path) = std::env::var("TOMCHAT_MODEL_PATH") {
            config.whisper.model_path = PathBuf::from(model_path);
        }
        
        if let Ok(hotkey) = std::env::var("TOMCHAT_HOTKEY") {
            config.hotkey.combination = hotkey;
        }
        
        // Expand relative paths to absolute
        if config.whisper.model_path.is_relative() {
            config.whisper.model_path = std::env::current_dir()?.join(&config.whisper.model_path);
        }
        
        // Fallback to existing model if configured model doesn't exist
        if !config.whisper.model_path.exists() {
            let fallback_path = PathBuf::from("/home/sujshe/src/whisper-hotkey-cpp/models/ggml-small.bin");
            if fallback_path.exists() {
                eprintln!("⚠️  Configured model not found: {:?}", config.whisper.model_path);
                eprintln!("🔄 Using fallback model: {:?}", fallback_path);
                config.whisper.model_path = fallback_path;
            }
        }
        
        Ok(config)
    }
}