use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::text_refinement::TextRefinementConfig;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub hotkey: HotkeyConfig,
    pub audio: AudioConfig,
    pub vad: VadConfig,
    pub speech: SpeechConfig,
    pub text: TextConfig,
    pub text_refinement: Option<TextRefinementConfig>,
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
    pub model_path: PathBuf,
    pub sensitivity: VadSensitivity,
    pub timeout_ms: u32,
    /// If true, auto-stop recording after silence timeout
    #[serde(default = "default_auto_stop")]
    pub auto_stop: bool,
}

fn default_auto_stop() -> bool {
    true
}

#[derive(Debug, Deserialize, Serialize)]
pub enum VadSensitivity {
    Low,
    Normal,
    High,
    VeryHigh,
}

impl VadSensitivity {
    pub fn to_threshold(&self) -> f32 {
        match self {
            VadSensitivity::Low => 0.3,
            VadSensitivity::Normal => 0.5,
            VadSensitivity::High => 0.7,
            VadSensitivity::VeryHigh => 0.85,
        }
    }

    // Keep for backward compatibility
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
pub struct SpeechConfig {
    /// Directory containing the Parakeet model files
    pub model_dir: PathBuf,
    pub language: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TextConfig {
    pub typing_delay_ms: u64,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = std::env::current_dir()?.join("config.toml");
        let config_str = std::fs::read_to_string(&config_path)?;
        let mut config: Config = toml::from_str(&config_str)?;

        // Override with environment variables if set
        if let Ok(model_dir) = std::env::var("TOMCHAT_MODEL_DIR") {
            config.speech.model_dir = PathBuf::from(model_dir);
        }

        if let Ok(hotkey) = std::env::var("TOMCHAT_HOTKEY") {
            config.hotkey.combination = hotkey;
        }

        // Expand relative paths to absolute
        let base_dir = std::env::current_dir()?;

        if config.speech.model_dir.is_relative() {
            config.speech.model_dir = base_dir.join(&config.speech.model_dir);
        }

        if config.vad.model_path.is_relative() {
            config.vad.model_path = base_dir.join(&config.vad.model_path);
        }

        // Validate model directory exists
        if !config.speech.model_dir.exists() {
            eprintln!("Model directory not found: {:?}", config.speech.model_dir);
            eprintln!("Run: scripts/download-parakeet.sh to download the model");
        }

        Ok(config)
    }
}
