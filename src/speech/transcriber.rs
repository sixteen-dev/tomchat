use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, debug};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

pub struct SpeechTranscriber {
    recognizer: Arc<RwLock<TransducerRecognizer>>,
    sample_rate: u32,
}

impl SpeechTranscriber {
    pub fn new<P: AsRef<Path>>(model_dir: P, _language: Option<&str>) -> Result<Self> {
        let model_path = model_dir.as_ref();
        info!("Loading Parakeet model from: {:?}", model_path);

        // Build paths to the ONNX model files
        let encoder_path = model_path.join("encoder.int8.onnx");
        let decoder_path = model_path.join("decoder.int8.onnx");
        let joiner_path = model_path.join("joiner.int8.onnx");
        let tokens_path = model_path.join("tokens.txt");

        // Verify files exist
        for path in [&encoder_path, &decoder_path, &joiner_path, &tokens_path] {
            if !path.exists() {
                return Err(anyhow::anyhow!(
                    "Model file not found: {:?}. Run scripts/download-parakeet.sh to download the model.",
                    path
                ));
            }
        }

        let config = TransducerConfig {
            encoder: encoder_path.to_string_lossy().to_string(),
            decoder: decoder_path.to_string_lossy().to_string(),
            joiner: joiner_path.to_string_lossy().to_string(),
            tokens: tokens_path.to_string_lossy().to_string(),
            num_threads: 4,  // Use multiple threads for faster inference
            sample_rate: 16_000,
            feature_dim: 80,
            debug: false,
            model_type: "nemo_transducer".to_string(),
            ..Default::default()
        };

        let recognizer = TransducerRecognizer::new(config)
            .map_err(|e| anyhow::anyhow!("Failed to create Parakeet recognizer: {}", e))?;

        info!("Parakeet model loaded successfully");

        Ok(Self {
            recognizer: Arc::new(RwLock::new(recognizer)),
            sample_rate: 16_000,
        })
    }

    pub async fn transcribe_audio(&self, audio_data: &[f32]) -> Result<String> {
        if audio_data.is_empty() {
            return Ok(String::new());
        }

        info!("Transcribing {} samples ({:.2}s of audio)",
              audio_data.len(),
              audio_data.len() as f32 / self.sample_rate as f32);

        let start = std::time::Instant::now();

        // Get mutable access to recognizer
        let mut recognizer = self.recognizer.write().await;

        // Transcribe - sherpa-rs expects f32 samples
        let result = recognizer.transcribe(self.sample_rate, audio_data);

        let elapsed = start.elapsed();
        let audio_duration = audio_data.len() as f32 / self.sample_rate as f32;
        let rtf = elapsed.as_secs_f32() / audio_duration;

        // Clean up result - lowercase and trim
        let cleaned = result.trim().to_string();

        info!("Transcription complete in {:.2}s (RTF: {:.2}x): \"{}\"",
              elapsed.as_secs_f32(), rtf, cleaned);

        Ok(cleaned)
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

    pub async fn get_model_info(&self) -> String {
        "Parakeet TDT 0.6B v2 (INT8 quantized)".to_string()
    }
}
