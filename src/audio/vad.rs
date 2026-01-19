use anyhow::Result;
use std::path::Path;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};

pub struct VoiceActivityDetector {
    vad: SileroVad,
    window_size: usize,
    sample_rate: u32,
    silence_timeout: Duration,
    last_speech_time: Option<Instant>,
    speech_detected: bool,
    pending_samples: Vec<f32>,
}

impl VoiceActivityDetector {
    pub fn new<P: AsRef<Path>>(
        model_path: P,
        sample_rate: u32,
        _sensitivity: i32,  // Silero doesn't use this the same way
        silence_timeout_ms: u32,
    ) -> Result<Self> {
        let model_path_str = model_path.as_ref().to_string_lossy().to_string();

        if !model_path.as_ref().exists() {
            return Err(anyhow::anyhow!(
                "VAD model not found: {}. Run scripts/download-parakeet.sh to download.",
                model_path_str
            ));
        }

        let window_size: usize = 512;  // Standard window size for Silero VAD

        let config = SileroVadConfig {
            model: model_path_str,
            window_size: window_size as i32,
            threshold: 0.5,  // Speech detection threshold
            min_silence_duration: 0.25,  // 250ms minimum silence
            min_speech_duration: 0.1,   // 100ms minimum speech
            ..Default::default()
        };

        // The second parameter is max_speech_duration in seconds
        let vad = SileroVad::new(config, 30.0)
            .map_err(|e| anyhow::anyhow!("Failed to initialize Silero VAD: {}", e))?;

        info!(
            "Silero VAD initialized: {}Hz, window_size: {}, silence_timeout: {}ms",
            sample_rate, window_size, silence_timeout_ms
        );

        Ok(Self {
            vad,
            window_size,
            sample_rate,
            silence_timeout: Duration::from_millis(silence_timeout_ms as u64),
            last_speech_time: None,
            speech_detected: false,
            pending_samples: Vec::new(),
        })
    }

    /// Process audio samples and return VAD result
    pub fn process_audio(&mut self, samples: &[f32]) -> VadResult {
        // Accumulate samples
        self.pending_samples.extend_from_slice(samples);

        let mut has_speech_in_frame = false;

        // Process complete windows
        while self.pending_samples.len() >= self.window_size {
            let window: Vec<f32> = self.pending_samples.drain(..self.window_size).collect();

            // Feed to Silero VAD
            self.vad.accept_waveform(window);

            // Check if speech detected
            if self.vad.is_speech() {
                has_speech_in_frame = true;
                self.last_speech_time = Some(Instant::now());

                if !self.speech_detected {
                    debug!("Speech detected");
                    self.speech_detected = true;
                }
            }
        }

        // Determine current state based on timeout
        let now = Instant::now();
        let silence_duration = self.last_speech_time
            .map(|t| now.duration_since(t))
            .unwrap_or(Duration::MAX);

        if self.speech_detected && silence_duration > self.silence_timeout {
            debug!("Silence timeout reached ({:?})", silence_duration);
            self.speech_detected = false;
            VadResult::SilenceDetected
        } else if has_speech_in_frame {
            VadResult::SpeechDetected
        } else {
            VadResult::Silence
        }
    }

    /// Reset VAD state for new recording session
    pub fn reset(&mut self) {
        self.vad.clear();
        self.pending_samples.clear();
        self.last_speech_time = None;
        self.speech_detected = false;
        debug!("VAD state reset");
    }

    /// Check if speech is currently active
    pub fn is_speech_active(&self) -> bool {
        self.speech_detected
    }

    /// Flush any remaining samples and finalize
    pub fn flush(&mut self) {
        self.vad.flush();
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum VadResult {
    /// Speech is currently being detected
    SpeechDetected,
    /// No speech detected in current frame (but may still be in speech session)
    Silence,
    /// Transition from speech to silence - timeout reached, recording should stop
    SilenceDetected,
}
