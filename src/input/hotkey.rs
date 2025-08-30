use anyhow::Result;
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

pub struct HotkeyManager {
    manager: GlobalHotKeyManager,
    hotkeys: HashMap<u32, String>,
}

#[allow(dead_code)]
impl HotkeyManager {
    pub fn new() -> Result<Self> {
        let manager = GlobalHotKeyManager::new()
            .map_err(|e| anyhow::anyhow!("Failed to create hotkey manager: {}", e))?;

        Ok(Self {
            manager,
            hotkeys: HashMap::new(),
        })
    }

    pub fn register_hotkey(&mut self, hotkey_string: &str) -> Result<u32> {
        let hotkey = parse_hotkey_string(hotkey_string)?;
        let id = hotkey.id();

        info!("Registering hotkey: {} (ID: {})", hotkey_string, id);

        self.manager
            .register(hotkey)
            .map_err(|e| anyhow::anyhow!("Failed to register hotkey '{}': {}", hotkey_string, e))?;

        self.hotkeys.insert(id, hotkey_string.to_string());

        info!("✅ Hotkey registered successfully: {}", hotkey_string);
        Ok(id)
    }

    pub fn unregister_hotkey(&mut self, id: u32) -> Result<()> {
        if let Some(hotkey_string) = self.hotkeys.remove(&id) {
            let hotkey = parse_hotkey_string(&hotkey_string)?;
            self.manager
                .unregister(hotkey)
                .map_err(|e| anyhow::anyhow!("Failed to unregister hotkey: {}", e))?;
            
            info!("Hotkey unregistered: {}", hotkey_string);
        }
        Ok(())
    }

    pub async fn start_listening(self, tx: mpsc::Sender<HotkeyEvent>) -> Result<()> {
        info!("🎯 Starting hotkey listener...");
        
        let receiver = GlobalHotKeyEvent::receiver();
        
        // Run the hotkey event loop
        tokio::task::spawn_blocking(move || {
            loop {
                if let Ok(event) = receiver.try_recv() {
                    let id = event.id;
                    match event.state {
                        global_hotkey::HotKeyState::Pressed => {
                            if let Some(hotkey_string) = self.hotkeys.get(&id) {
                                debug!("🔑 Hotkey pressed: {} (ID: {})", hotkey_string, id);
                                
                                let event = HotkeyEvent {
                                    id,
                                    hotkey: hotkey_string.clone(),
                                    pressed: true,
                                };
                                
                                if let Err(_) = tx.blocking_send(event) {
                                    error!("Failed to send hotkey event - receiver dropped");
                                    break;
                                }
                            }
                        }
                        global_hotkey::HotKeyState::Released => {
                            if let Some(hotkey_string) = self.hotkeys.get(&id) {
                                debug!("🔑 Hotkey released: {} (ID: {})", hotkey_string, id);
                                
                                let event = HotkeyEvent {
                                    id,
                                    hotkey: hotkey_string.clone(),
                                    pressed: false,
                                };
                                
                                if let Err(_) = tx.blocking_send(event) {
                                    error!("Failed to send hotkey event - receiver dropped");
                                    break;
                                }
                            }
                        }
                    }
                }
                
                // Small sleep to prevent busy waiting
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }).await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct HotkeyEvent {
    pub id: u32,
    #[allow(dead_code)]
    pub hotkey: String,
    pub pressed: bool,
}

fn parse_hotkey_string(hotkey_string: &str) -> Result<HotKey> {
    let parts: Vec<&str> = hotkey_string.split('+').map(|s| s.trim()).collect();
    
    if parts.is_empty() {
        return Err(anyhow::anyhow!("Empty hotkey string"));
    }
    
    let mut modifiers = Modifiers::empty();
    let mut key_code = None;
    
    for part in parts {
        match part.to_lowercase().as_str() {
            "ctrl" | "cmd" => modifiers |= Modifiers::CONTROL,
            "shift" => modifiers |= Modifiers::SHIFT,
            "alt" => modifiers |= Modifiers::ALT,
            "super" | "win" | "meta" => modifiers |= Modifiers::SUPER,
            key => {
                if key_code.is_some() {
                    return Err(anyhow::anyhow!("Multiple keys specified in hotkey: {}", hotkey_string));
                }
                key_code = Some(parse_key_code(key)?);
            }
        }
    }
    
    let key_code = key_code.unwrap_or(Code::Unidentified);
    
    Ok(HotKey::new(Some(modifiers), key_code))
}

fn parse_key_code(key: &str) -> Result<Code> {
    match key.to_lowercase().as_str() {
        // Letters
        "a" => Ok(Code::KeyA),
        "b" => Ok(Code::KeyB),
        "c" => Ok(Code::KeyC),
        "d" => Ok(Code::KeyD),
        "e" => Ok(Code::KeyE),
        "f" => Ok(Code::KeyF),
        "g" => Ok(Code::KeyG),
        "h" => Ok(Code::KeyH),
        "i" => Ok(Code::KeyI),
        "j" => Ok(Code::KeyJ),
        "k" => Ok(Code::KeyK),
        "l" => Ok(Code::KeyL),
        "m" => Ok(Code::KeyM),
        "n" => Ok(Code::KeyN),
        "o" => Ok(Code::KeyO),
        "p" => Ok(Code::KeyP),
        "q" => Ok(Code::KeyQ),
        "r" => Ok(Code::KeyR),
        "s" => Ok(Code::KeyS),
        "t" => Ok(Code::KeyT),
        "u" => Ok(Code::KeyU),
        "v" => Ok(Code::KeyV),
        "w" => Ok(Code::KeyW),
        "x" => Ok(Code::KeyX),
        "y" => Ok(Code::KeyY),
        "z" => Ok(Code::KeyZ),
        
        // Numbers
        "0" => Ok(Code::Digit0),
        "1" => Ok(Code::Digit1),
        "2" => Ok(Code::Digit2),
        "3" => Ok(Code::Digit3),
        "4" => Ok(Code::Digit4),
        "5" => Ok(Code::Digit5),
        "6" => Ok(Code::Digit6),
        "7" => Ok(Code::Digit7),
        "8" => Ok(Code::Digit8),
        "9" => Ok(Code::Digit9),
        
        // Special keys
        "space" => Ok(Code::Space),
        "enter" | "return" => Ok(Code::Enter),
        "tab" => Ok(Code::Tab),
        "backspace" => Ok(Code::Backspace),
        "delete" => Ok(Code::Delete),
        "escape" | "esc" => Ok(Code::Escape),
        "lshift" => Ok(Code::ShiftLeft),
        "rshift" => Ok(Code::ShiftRight),
        
        // Function keys
        "f1" => Ok(Code::F1),
        "f2" => Ok(Code::F2),
        "f3" => Ok(Code::F3),
        "f4" => Ok(Code::F4),
        "f5" => Ok(Code::F5),
        "f6" => Ok(Code::F6),
        "f7" => Ok(Code::F7),
        "f8" => Ok(Code::F8),
        "f9" => Ok(Code::F9),
        "f10" => Ok(Code::F10),
        "f11" => Ok(Code::F11),
        "f12" => Ok(Code::F12),
        
        // Arrow keys
        "up" => Ok(Code::ArrowUp),
        "down" => Ok(Code::ArrowDown),
        "left" => Ok(Code::ArrowLeft),
        "right" => Ok(Code::ArrowRight),
        
        _ => Err(anyhow::anyhow!("Unknown key: {}", key)),
    }
}