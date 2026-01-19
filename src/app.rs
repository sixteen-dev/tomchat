use anyhow::Result;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tracing::{error, info, debug, warn};

use crate::audio::{AudioCapture, VoiceActivityDetector, VadResult};
use crate::config::Config;
use crate::input::{HotkeyEvent, HotkeyManager, TextInjector};
use crate::speech::SpeechTranscriber;
use crate::text_refinement::TextRefiner;

pub struct TomChatApp {
    config: Config,
    audio_capture: AudioCapture,
    vad: VoiceActivityDetector,
    transcriber: SpeechTranscriber,
    text_refiner: Option<TextRefiner>,
    text_injector: TextInjector,
    hotkey_manager: HotkeyManager,
    gui_mode: bool,
    test_mode: bool,
}

impl TomChatApp {
    pub async fn new(config: Config) -> Result<Self> {
        info!("Initializing TomChat...");

        // Initialize audio capture
        let audio_capture = AudioCapture::new()?;

        // Initialize Silero VAD
        let vad = VoiceActivityDetector::new(
            &config.vad.model_path,
            config.audio.sample_rate,
            config.vad.sensitivity.to_webrtc_mode(),
            config.vad.timeout_ms,
        )?;

        // Initialize Parakeet transcriber
        let transcriber = SpeechTranscriber::new(
            &config.speech.model_dir,
            Some(&config.speech.language),
        )?;

        // Initialize text refiner (optional)
        let text_refiner = if let Some(ref refinement_config) = config.text_refinement {
            if refinement_config.enabled {
                match TextRefiner::new(refinement_config.clone()).await {
                    Ok(refiner) => {
                        info!("Text refinement initialized");
                        Some(refiner)
                    }
                    Err(e) => {
                        warn!("Text refinement failed: {}, continuing without", e);
                        None
                    }
                }
            } else {
                debug!("Text refinement disabled");
                None
            }
        } else {
            None
        };

        // Initialize text injector
        let text_injector = TextInjector::new(config.text.typing_delay_ms)?;

        // Initialize hotkey manager
        let hotkey_manager = HotkeyManager::new()?;

        info!("All components initialized successfully");

        Ok(Self {
            config,
            audio_capture,
            vad,
            transcriber,
            text_refiner,
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

    fn notify_state_change(recording: bool) {
        info!("State change: recording={}", recording);

        // Write state to file for Tauri app to read
        let state_update = serde_json::json!({
            "recording": recording,
            "timestamp": std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        });

        let state_file = "/tmp/tomchat_bubble_state.json";
        match std::fs::write(state_file, state_update.to_string()) {
            Ok(_) => debug!("State update written to file: recording={}", recording),
            Err(e) => error!("Failed to write state file: {}", e),
        }
    }

    pub async fn run(mut self) -> Result<()> {
        info!("Starting TomChat application...");

        let gui_mode = self.gui_mode;
        let vad_auto_stop = self.config.vad.auto_stop;

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
        let vad = Arc::new(Mutex::new(self.vad));

        // Register hotkey
        let id = self.hotkey_manager.register_hotkey(&self.config.hotkey.combination)?;
        info!("Hotkey registered: {}", self.config.hotkey.combination);
        let hotkey_id = id;

        // Start audio capture
        self.audio_capture.start_capture(audio_tx).await?;

        // Clone references for async tasks
        let transcriber_clone = Arc::new(self.transcriber);
        let recording_state_clone = recording_state.clone();
        let audio_buffer_clone = audio_buffer.clone();
        let transcription_tx_clone = transcription_tx.clone();
        let emit_status_audio = emit_status.clone();
        let vad_clone = vad.clone();
        let process_tx_clone = process_tx.clone();

        // Audio processing task with VAD auto-stop
        let audio_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    // Handle audio chunks
                    Some(audio_chunk) = audio_rx.recv() => {
                        let mut state = recording_state_clone.lock().await;

                        if !state.is_recording {
                            continue; // Skip processing when not recording
                        }

                        // Add to audio buffer
                        {
                            let mut buffer = audio_buffer_clone.lock().await;
                            buffer.extend(&audio_chunk);
                        }

                        // Process VAD for auto-stop
                        if vad_auto_stop {
                            let mut vad = vad_clone.lock().await;
                            let vad_result = vad.process_audio(&audio_chunk);

                            match vad_result {
                                VadResult::SpeechDetected => {
                                    if !state.speech_detected {
                                        debug!("Speech started");
                                        state.speech_detected = true;
                                    }
                                }
                                VadResult::SilenceDetected => {
                                    // Auto-stop: silence timeout reached after speech
                                    if state.speech_detected {
                                        info!("Auto-stopping: silence detected after speech");
                                        state.is_recording = false;
                                        state.speech_detected = false;

                                        // Notify state change
                                        TomChatApp::notify_state_change(false);

                                        // Trigger transcription
                                        let _ = process_tx_clone.send(()).await;
                                    }
                                }
                                VadResult::Silence => {
                                    // Still waiting for speech or in between words
                                }
                            }
                        }
                    }

                    // Handle process signal (when recording stops)
                    Some(_) = process_rx.recv() => {
                        info!("Processing audio...");

                        // Reset VAD for next session
                        {
                            let mut vad = vad_clone.lock().await;
                            vad.reset();
                        }

                        // Get accumulated audio
                        let audio_data = {
                            let mut buffer = audio_buffer_clone.lock().await;
                            let data: Vec<f32> = buffer.iter().cloned().collect();
                            buffer.clear();
                            data
                        };

                        // Send for transcription
                        if !audio_data.is_empty() {
                            info!("Transcribing {} audio samples ({:.1}s)",
                                  audio_data.len(),
                                  audio_data.len() as f32 / 16000.0);
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
                                        debug!("Empty transcription result");
                                    }
                                    Err(e) => {
                                        emit_clone("transcription_error", &format!("Transcription failed: {}", e));
                                        error!("Transcription failed: {}", e);
                                    }
                                }
                            });
                        } else {
                            info!("No audio data to transcribe");
                        }
                    }
                }
            }
        });

        // Transcription handling task
        let mut text_injector = self.text_injector;
        let text_refiner_clone = self.text_refiner;
        let transcription_task = tokio::spawn(async move {
            while let Some(raw_text) = transcription_rx.recv().await {
                info!("Transcribed: \"{}\"", raw_text);

                // Apply text refinement if enabled
                let final_text = if let Some(ref refiner) = text_refiner_clone {
                    match refiner.refine_text(&raw_text).await {
                        Ok(refined_text) => {
                            if refined_text != raw_text {
                                info!("Refined: \"{}\" -> \"{}\"", raw_text, refined_text);
                            }
                            refined_text
                        }
                        Err(e) => {
                            warn!("Text refinement failed: {}, using original", e);
                            raw_text
                        }
                    }
                } else {
                    raw_text
                };

                if let Err(e) = text_injector.inject_with_formatting(&final_text).await {
                    error!("Failed to inject text: {}", e);
                } else {
                    info!("Text injected successfully");
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

        // Clone emit_status for main loop
        let emit_status_hotkey = emit_status.clone();
        let vad_main = vad.clone();

        // Main event loop
        let main_task = tokio::spawn(async move {
            while let Some(hotkey_event) = hotkey_rx.recv().await {
                if hotkey_event.pressed && hotkey_event.id == hotkey_id {
                    let mut state = recording_state_hotkey.lock().await;

                    if !state.is_recording {
                        info!("Recording started by hotkey");
                        emit_status_hotkey("recording_started", "Recording started");

                        // Reset VAD for new session
                        {
                            let mut vad = vad_main.lock().await;
                            vad.reset();
                        }

                        state.is_recording = true;
                        state.speech_detected = false;

                        // Notify bubble of state change
                        TomChatApp::notify_state_change(true);
                    } else {
                        info!("Recording stopped by hotkey");
                        emit_status_hotkey("recording_stopped", "Recording stopped");
                        state.is_recording = false;
                        state.speech_detected = false;

                        // Notify bubble of state change
                        TomChatApp::notify_state_change(false);

                        // Signal audio processing to transcribe accumulated audio
                        if let Err(_) = process_tx.send(()).await {
                            error!("Failed to send process signal");
                        }
                    }
                }
            }
        });

        info!("TomChat is ready!");
        info!("Press {} to start recording", self.config.hotkey.combination);
        if vad_auto_stop {
            info!("Auto-stop enabled: recording will stop after {}ms of silence",
                  self.config.vad.timeout_ms);
        } else {
            info!("Press {} again to stop recording", self.config.hotkey.combination);
        }
        info!("Press Ctrl+C to exit");

        // Wait for any task to complete (or error)
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

        info!("TomChat shutting down gracefully...");
        Ok(())
    }
}

#[derive(Debug, Default)]
struct RecordingState {
    is_recording: bool,
    speech_detected: bool,
}
