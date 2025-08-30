use anyhow::Result;
use std::collections::VecDeque;
use std::time::{Duration, Instant};
use tracing::{debug, info};
use webrtc_vad::{SampleRate, Vad, VadMode};

#[allow(dead_code)]
pub struct VoiceActivityDetector {
    vad: Vad,
    sample_rate: SampleRate,
    frame_size: usize,
    buffer: VecDeque<f32>,
    silence_timeout: Duration,
    last_speech_time: Option<Instant>,
    speech_detected: bool,
}

#[allow(dead_code)]
impl VoiceActivityDetector {
    pub fn new(sample_rate: u32, sensitivity: i32, silence_timeout_ms: u32) -> Result<Self> {
        let sample_rate_enum = match sample_rate {
            8000 => SampleRate::Rate8kHz,
            16000 => SampleRate::Rate16kHz,
            32000 => SampleRate::Rate32kHz,
            48000 => SampleRate::Rate48kHz,
            _ => return Err(anyhow::anyhow!("Unsupported sample rate: {}", sample_rate)),
        };

        let vad_mode = match sensitivity {
            0 => VadMode::Quality,
            1 => VadMode::LowBitrate,
            2 => VadMode::Aggressive,
            3 => VadMode::VeryAggressive,
            _ => VadMode::Quality,
        };

        let mut vad = Vad::new();
        vad.set_mode(vad_mode);

        // Frame size must be 10, 20, or 30ms worth of samples
        let frame_size = match sample_rate {
            8000 => 160,   // 20ms at 8kHz
            16000 => 320,  // 20ms at 16kHz  
            32000 => 640,  // 20ms at 32kHz
            48000 => 960,  // 20ms at 48kHz
            _ => unreachable!(),
        };

        info!(
            "WebRTC VAD initialized: {}Hz, mode: Quality/LowBitrate/Aggressive/VeryAggressive, frame_size: {}",
            sample_rate, frame_size
        );

        Ok(Self {
            vad,
            sample_rate: sample_rate_enum,
            frame_size,
            buffer: VecDeque::new(),
            silence_timeout: Duration::from_millis(silence_timeout_ms as u64),
            last_speech_time: None,
            speech_detected: false,
        })
    }

    pub fn process_audio(&mut self, samples: &[f32]) -> VadResult {
        // Add samples to buffer
        self.buffer.extend(samples);

        let mut has_speech = false;
        let mut _processed_frames = 0;

        // Process complete frames
        while self.buffer.len() >= self.frame_size {
            let frame: Vec<f32> = self.buffer.drain(..self.frame_size).collect();
            
            // Convert f32 to i16 for WebRTC VAD
            let frame_i16: Vec<i16> = frame
                .iter()
                .map(|&sample| (sample.clamp(-1.0, 1.0) * 32767.0) as i16)
                .collect();

            // Run VAD on this frame
            match self.vad.is_voice_segment(&frame_i16) {
                Ok(is_speech) => {
                    if is_speech {
                        has_speech = true;
                        self.last_speech_time = Some(Instant::now());
                        
                        if !self.speech_detected {
                            debug!("🎤 Speech detected");
                            self.speech_detected = true;
                        }
                    }
                    _processed_frames += 1;
                }
                Err(_e) => {
                    debug!("VAD processing error occurred");
                    continue;
                }
            }
        }

        // Determine current state
        let now = Instant::now();
        let is_silent = if let Some(last_speech) = self.last_speech_time {
            now.duration_since(last_speech) > self.silence_timeout
        } else {
            true
        };

        if is_silent && self.speech_detected {
            debug!("🔇 Silence timeout reached");
            self.speech_detected = false;
            VadResult::SilenceDetected
        } else if has_speech {
            VadResult::SpeechDetected
        } else {
            VadResult::Silence
        }
    }

    pub fn reset(&mut self) {
        self.buffer.clear();
        self.last_speech_time = None;
        self.speech_detected = false;
        debug!("VAD state reset");
    }

    pub fn is_speech_active(&self) -> bool {
        self.speech_detected
    }
}

#[derive(Debug, Clone, PartialEq)]
#[allow(dead_code)]
pub enum VadResult {
    SpeechDetected,
    Silence,
    SilenceDetected, // Transition from speech to silence (timeout)
}