use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, debug};
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

use whisper_rs::WhisperState;

pub struct SpeechTranscriber {
    context: Arc<RwLock<WhisperContext>>,
    state: Arc<RwLock<WhisperState>>, // Reuse state instead of creating new one
}

#[allow(dead_code)]
impl SpeechTranscriber {
    pub fn new<P: AsRef<Path>>(model_path: P, _language: Option<&str>) -> Result<Self> {
        info!("Loading Whisper model from: {:?}", model_path.as_ref());
        
        // Load the Whisper model
        let ctx = WhisperContext::new_with_params(
            model_path.as_ref().to_str().unwrap(),
            WhisperContextParameters::default(),
        )
        .map_err(|e| anyhow::anyhow!("Failed to load Whisper model: {}", e))?;

        info!("Whisper model loaded successfully");

        // Create state once during initialization
        let state = ctx.create_state()
            .map_err(|e| anyhow::anyhow!("Failed to create Whisper state: {}", e))?;

        Ok(Self {
            context: Arc::new(RwLock::new(ctx)),
            state: Arc::new(RwLock::new(state)),
        })
    }

    pub async fn transcribe_audio(&self, audio_data: &[f32]) -> Result<String> {
        if audio_data.is_empty() {
            return Ok(String::new());
        }

        info!("🎯 Transcribing {} samples", audio_data.len());

        // Convert f32 samples to i16 format (like the working example) - no normalization
        let i16_samples: Vec<i16> = audio_data.iter()
            .map(|&sample| (sample * 32767.0).clamp(-32768.0, 32767.0) as i16)
            .collect();
            
        // Convert i16 to f32 using whisper-rs proper conversion
        let mut converted_samples = vec![0.0f32; i16_samples.len()];
        whisper_rs::convert_integer_to_float_audio(&i16_samples, &mut converted_samples)
            .map_err(|e| anyhow::anyhow!("Failed to convert audio: {}", e))?;

        // Reuse the pre-created state (much faster!)
        let mut state = self.state.write().await;

        // Speed-optimized params for small.en model
        let mut params = FullParams::new(SamplingStrategy::Greedy { 
            best_of: 1        // Single best for speed
        });
        
        // Minimal params for maximum speed
        params.set_suppress_blank(true);        
        params.set_temperature(0.0);            
        params.set_language(Some("en"));        
        params.set_translate(false);
        params.set_token_timestamps(false);     // Disable timestamps for speed
        
        // Run the transcription with reused state (no initialization overhead!)
        state.full(params, &converted_samples)
            .map_err(|e| anyhow::anyhow!("Transcription failed: {}", e))?;

        // Extract transcribed text
        let num_segments = state.full_n_segments();
        let mut result = String::new();
        
        for i in 0..num_segments {
            if let Some(segment) = state.get_segment(i) {
                let text = segment.to_string().trim().to_string();
                if !text.is_empty() {
                    if !result.is_empty() {
                        result.push(' ');
                    }
                    result.push_str(&text);
                }
            }
        }
        
        info!("📝 Transcription: \"{}\"", result);
        Ok(result)
    }

    pub async fn transcribe_streaming(&self, audio_chunks: Vec<Vec<f32>>) -> Result<Vec<String>> {
        let mut results = Vec::new();
        
        for chunk in audio_chunks {
            match self.transcribe_audio(&chunk).await {
                Ok(text) => {
                    if !text.is_empty() {
                        results.push(text);
                    }
                }
                Err(e) => {
                    error!("Streaming transcription error: {}", e);
                }
            }
        }
        
        Ok(results)
    }

    // Get model information
    pub async fn get_model_info(&self) -> String {
        let _ctx = self.context.read().await;
        format!("Whisper model loaded, vocab size: (info not available in current API)")
    }
}

// Helper function to calculate RMS (Root Mean Square) for audio quality assessment
fn calculate_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_of_squares: f32 = samples.iter().map(|&s| s * s).sum();
    (sum_of_squares / samples.len() as f32).sqrt()
}