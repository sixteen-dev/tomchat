use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, debug, warn};
use std::fs;

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
    gui_mode: bool,
    test_mode: bool,
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
            gui_mode: false,
            test_mode: false,
        })
    }
    
    pub fn set_gui_mode(&mut self, gui_mode: bool) {
        self.gui_mode = gui_mode;
    }
    
    pub fn set_test_mode(&mut self, test_mode: bool) {
        self.test_mode = test_mode;
    }
    
    // Emit JSON status event to stdout when in GUI mode
    fn emit_status(&self, event: &str, message: &str) {
        if self.gui_mode {
            let json = serde_json::json!({
                "event": event,
                "message": message,
                "timestamp": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs()
            });
            println!("{}", json);
        }
    }

    async fn notify_state_change(recording: bool) {
        info!("State change: recording={}", recording);
        
        // Send state update to Tauri HTTP server
        let client = reqwest::Client::new();
        let state_update = serde_json::json!({
            "recording": recording,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });
        
        let result = client
            .post("http://localhost:8081/state")
            .json(&state_update)
            .send()
            .await;
            
        match result {
            Ok(_) => {
                info!("State update sent to bubble via HTTP: recording={}", recording);
            }
            Err(e) => {
                warn!("HTTP request failed: {}", e);
                
                // Fallback: write state to file
                let state_update = serde_json::json!({
                    "recording": recording,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });
                
                let state_file = "/tmp/tomchat_bubble_state.json";
                match std::fs::write(state_file, state_update.to_string()) {
                    Ok(_) => {
                        info!("State written to file as fallback: recording={}", recording);
                    }
                    Err(file_err) => {
                        error!("All communication methods failed: HTTP={}, File={}", e, file_err);
                    }
                }
            }
        }
    }

    pub async fn run(mut self) -> Result<()> {
        info!("🚀 Starting TomChat application...");
        
        let gui_mode = self.gui_mode;
        
        // Helper function to emit status events (shareable)
        let emit_status = Arc::new(move |event: &str, message: &str| {
            if gui_mode {
                let json = serde_json::json!({
                    "event": event,
                    "message": message,
                    "timestamp": std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                });
                println!("{}", json);
            }
        });

        // Create communication channels
        let (audio_tx, mut audio_rx) = mpsc::unbounded_channel::<Vec<f32>>();
        let (hotkey_tx, mut hotkey_rx) = mpsc::channel::<HotkeyEvent>(100);
        let (transcription_tx, mut transcription_rx) = mpsc::channel::<String>(100);
        let (process_tx, mut process_rx) = mpsc::channel::<()>(10);

        // Shared state for recording
        let recording_state = Arc::new(Mutex::new(RecordingState::default()));
        let audio_buffer = Arc::new(Mutex::new(VecDeque::<f32>::new()));

        // Register hotkey (always register, even in GUI mode)
        let id = self.hotkey_manager.register_hotkey(&self.config.hotkey.combination)?;
        info!("🔑 Hotkey registered: {}", self.config.hotkey.combination);
        let hotkey_id = id;

        // Start audio capture
        self.audio_capture.start_capture(audio_tx).await?;

        // Clone references for async tasks
        let transcriber_clone = Arc::new(self.transcriber);
        let recording_state_clone = recording_state.clone();
        let audio_buffer_clone = audio_buffer.clone();
        let transcription_tx_clone = transcription_tx.clone();
        let emit_status_audio = emit_status.clone();

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
                            emit_status_audio("transcribing", "Transcribing audio");
                            let transcriber = transcriber_clone.clone();
                            let tx = transcription_tx_clone.clone();
                            let emit_clone = emit_status_audio.clone();
                            
                            tokio::spawn(async move {
                                match transcriber.transcribe_audio(&audio_data).await {
                                    Ok(text) if !text.is_empty() => {
                                        emit_clone("transcription_complete", &format!("Transcription: {}", text));
                                        if let Err(_) = tx.send(text).await {
                                            error!("Failed to send transcription");
                                        }
                                    }
                                    Ok(_) => {
                                        emit_clone("transcription_complete", "Empty transcription result");
                                        debug!("Empty transcription result")
                                    },
                                    Err(e) => {
                                        emit_clone("transcription_error", &format!("Transcription failed: {}", e));
                                        error!("Transcription failed: {}", e)
                                    },
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
        let hotkey_task = if self.gui_mode {
            // In GUI mode, create a dummy task that does nothing
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(u64::MAX)).await;
                Ok(())
            })
        } else {
            tokio::spawn(async move {
                self.hotkey_manager.start_listening(hotkey_tx).await
            })
        };

        // Clone emit_status for async tasks
        let emit_status_hotkey = emit_status.clone();
        
        // Test mode: simulate recording events
        if self.test_mode {
            let emit_test = emit_status.clone();
            let recording_state_test = recording_state.clone();
            let process_tx_test = process_tx.clone();
            
            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                
                loop {
                    // Start recording
                    emit_test("recording_started", "Test recording started");
                    {
                        let mut state = recording_state_test.lock().await;
                        state.is_recording = true;
                    }
                    
                    // Record for 3 seconds
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    
                    // Stop recording
                    emit_test("recording_stopped", "Test recording stopped");
                    {
                        let mut state = recording_state_test.lock().await;
                        state.is_recording = false;
                    }
                    
                    // Trigger transcription
                    if let Err(_) = process_tx_test.send(()).await {
                        break;
                    }
                    
                    // Wait before next cycle
                    tokio::time::sleep(std::time::Duration::from_secs(10)).await;
                }
            });
        }
        
        // Main event loop
        let main_task = tokio::spawn(async move {
            while let Some(hotkey_event) = hotkey_rx.recv().await {
                if hotkey_event.pressed && hotkey_event.id == hotkey_id {
                    let mut state = recording_state_hotkey.lock().await;
                    
                    if !state.is_recording {
                        info!("🎤 Recording started by hotkey");
                        emit_status_hotkey("recording_started", "Recording started");
                        state.is_recording = true;
                        state.speech_detected = false;
                        
                        // Notify bubble of state change
                        tokio::spawn(async {
                            TomChatApp::notify_state_change(true).await;
                        });
                    } else {
                        info!("⏹️ Recording stopped by hotkey");
                        emit_status_hotkey("recording_stopped", "Recording stopped");
                        state.is_recording = false;
                        state.speech_detected = false;
                        
                        // Notify bubble of state change
                        tokio::spawn(async {
                            TomChatApp::notify_state_change(false).await;
                        });
                        
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