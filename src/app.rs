use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, debug};

use crate::audio::{AudioCapture, VoiceActivityDetector};
use crate::config::Config;
use crate::input::{HotkeyEvent, HotkeyManager, TextInjector};
use crate::speech::SpeechTranscriber;

pub struct TomChatApp {
    config: Config,
    audio_capture: AudioCapture,
    #[allow(dead_code)]
    vad: VoiceActivityDetector,
    transcriber: SpeechTranscriber,
    text_injector: TextInjector,
    hotkey_manager: HotkeyManager,
}

impl TomChatApp {
    pub async fn new(config: Config) -> Result<Self> {
        info!("🚀 Initializing TomChat (named after Tommy)...");

        // Initialize audio capture
        let audio_capture = AudioCapture::new()?;

        // Initialize VAD with config settings
        let vad = VoiceActivityDetector::new(
            config.audio.sample_rate,
            config.vad.sensitivity.to_webrtc_mode(),
            config.vad.timeout_ms,
        )?;

        // Initialize Whisper transcriber
        let transcriber = SpeechTranscriber::new(
            &config.whisper.model_path,
            Some(&config.whisper.language),
        )?;

        // Initialize text injector
        let text_injector = TextInjector::new(config.text.typing_delay_ms)?;

        // Initialize hotkey manager
        let hotkey_manager = HotkeyManager::new()?;

        info!("✅ All components initialized successfully");

        Ok(Self {
            config,
            audio_capture,
            vad,
            transcriber,
            text_injector,
            hotkey_manager,
        })
    }

    pub async fn run(mut self) -> Result<()> {
        info!("🚀 Starting TomChat application...");

        // Create communication channels
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
        let (hotkey_tx, mut hotkey_rx) = mpsc::channel::<HotkeyEvent>(100);
        let (transcription_tx, mut transcription_rx) = mpsc::channel::<String>(100);
        let (process_tx, mut process_rx) = mpsc::channel::<()>(10);

        // Shared state for recording
        let recording_state = Arc::new(Mutex::new(RecordingState::default()));
        let audio_buffer = Arc::new(Mutex::new(VecDeque::<f32>::new()));

        // Register hotkey
        let hotkey_id = self.hotkey_manager.register_hotkey(&self.config.hotkey.combination)?;
        info!("=% Hotkey registered: {}", self.config.hotkey.combination);

        // Start audio capture
        self.audio_capture.start_capture(audio_tx).await?;

        // Clone references for async tasks
        let transcriber_clone = Arc::new(self.transcriber);
        let recording_state_clone = recording_state.clone();
        let audio_buffer_clone = audio_buffer.clone();
        let transcription_tx_clone = transcription_tx.clone();

        // Audio processing task - manual control mode
        let audio_task = tokio::spawn(async move {
            
            loop {
                tokio::select! {
                    // Handle audio chunks
                    Some(audio_chunk) = audio_rx.recv() => {
                        let state = recording_state_clone.lock().await;
                        
                        
                        if !state.is_recording {
                            continue; // Skip processing when not recording
                        }

                        // Add to audio buffer
                        {
                            let mut buffer = audio_buffer_clone.lock().await;
                            buffer.extend(&audio_chunk);
                        }
                        
                        // Manual recording - just accumulate audio while recording
                    }
                    
                    // Handle process signal (when recording stops manually)
                    Some(_) = process_rx.recv() => {
                        info!("🔇 Processing audio after manual stop");
                        
                        // Get accumulated audio
                        let audio_data = {
                            let mut buffer = audio_buffer_clone.lock().await;
                            let data: Vec<f32> = buffer.iter().cloned().collect();
                            buffer.clear();
                            data
                        };

                        // Send for transcription
                        if !audio_data.is_empty() {
                            info!("📝 Transcribing {} audio samples", audio_data.len());
                            let transcriber = transcriber_clone.clone();
                            let tx = transcription_tx_clone.clone();
                            
                            tokio::spawn(async move {
                                match transcriber.transcribe_audio(&audio_data).await {
                                    Ok(text) if !text.is_empty() => {
                                        if let Err(_) = tx.send(text).await {
                                            error!("Failed to send transcription");
                                        }
                                    }
                                    Ok(_) => debug!("Empty transcription result"),
                                    Err(e) => error!("Transcription failed: {}", e),
                                }
                            });
                        } else {
                            info!("⚠️ No audio data to transcribe");
                        }
                        
                    }
                }
            }
        });

        // Helper function to calculate RMS (Root Mean Square) for voice activity detection
        fn calculate_rms(samples: &[f32]) -> f32 {
            if samples.is_empty() {
                return 0.0;
            }
            let sum_of_squares: f32 = samples.iter().map(|&s| s * s).sum();
            (sum_of_squares / samples.len() as f32).sqrt()
        }


        // Transcription handling task
        let mut text_injector = self.text_injector;
        let transcription_task = tokio::spawn(async move {
            while let Some(text) = transcription_rx.recv().await {
                info!("📝 Transcribed: \"{}\"", text);
                
                if let Err(e) = text_injector.inject_with_formatting(&text).await {
                    error!("Failed to inject text: {}", e);
                } else {
                    info!("✅ Text injected successfully");
                }
            }
        });

        // Hotkey handling task  
        let recording_state_hotkey = recording_state.clone();
        let hotkey_task = tokio::spawn(async move {
            self.hotkey_manager.start_listening(hotkey_tx).await
        });

        // Main event loop
        let main_task = tokio::spawn(async move {
            while let Some(hotkey_event) = hotkey_rx.recv().await {
                if hotkey_event.pressed && hotkey_event.id == hotkey_id {
                    let mut state = recording_state_hotkey.lock().await;
                    
                    if !state.is_recording {
                        info!("🎤 Recording started by hotkey");
                        state.is_recording = true;
                        state.speech_detected = false;
                    } else {
                        info!("⏹️ Recording stopped by hotkey");
                        state.is_recording = false;
                        state.speech_detected = false;
                        
                        // Signal audio processing to transcribe accumulated audio
                        if let Err(_) = process_tx.send(()).await {
                            error!("Failed to send process signal");
                        }
                    }
                }
            }
        });

        info!("🚀 TomChat is ready! Press {} to start/stop recording.", self.config.hotkey.combination);
        info!("Press Ctrl+C to exit.");

        // Wait for any task to complete (or error)
        // The main.rs handles Ctrl+C and will drop this future, causing tasks to be cancelled
        tokio::select! {
            result = audio_task => {
                if let Err(e) = result {
                    error!("Audio task failed: {}", e);
                }
            }
            result = transcription_task => {
                if let Err(e) = result {
                    error!("Transcription task failed: {}", e);
                }
            }
            result = hotkey_task => {
                if let Err(e) = result {
                    error!("Hotkey task failed: {}", e);
                }
            }
            result = main_task => {
                if let Err(e) = result {
                    error!("Main task failed: {}", e);
                }
            }
        }

        info!("⏹️ TomChat shutting down gracefully...");
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RecordingState {
    is_recording: bool,
    speech_detected: bool,
}