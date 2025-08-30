use anyhow::Result;
use enigo::{Enigo, Key, Settings, Direction, Keyboard};
use std::time::Duration;
use tracing::{debug, info};

pub struct TextInjector {
    enigo: Enigo,
    #[allow(dead_code)]
    typing_delay: Duration,
}

#[allow(dead_code)]
impl TextInjector {
    pub fn new(typing_delay_ms: u64) -> Result<Self> {
        let settings = Settings::default();
        let enigo = Enigo::new(&settings)
            .map_err(|e| anyhow::anyhow!("Failed to initialize text injector: {}", e))?;

        info!("📝 Text injector initialized with {}ms typing delay", typing_delay_ms);

        Ok(Self {
            enigo,
            typing_delay: Duration::from_millis(typing_delay_ms),
        })
    }

    pub async fn inject_text(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        info!("📝 Injecting text: \"{}\"", text);

        // Small delay to ensure target application is ready
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Type the text character by character with delays
        for char in text.chars() {
            self.type_character(char)?;
            
            // Add delay between characters if configured
            if !self.typing_delay.is_zero() {
                tokio::time::sleep(self.typing_delay).await;
            }
        }

        debug!("✅ Text injection completed");
        Ok(())
    }

    pub async fn inject_text_fast(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        info!("🚀 Fast injecting text: \"{}\"", text);

        // Small delay to ensure target application is ready
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Type the entire string at once (faster)
        self.enigo
            .text(text)
            .map_err(|e| anyhow::anyhow!("Failed to inject text: {}", e))?;

        debug!("✅ Fast text injection completed");
        Ok(())
    }

    fn type_character(&mut self, c: char) -> Result<()> {
        match c {
            // Special characters that need specific handling
            '\n' => {
                self.enigo.key(Key::Return, Direction::Click)
                    .map_err(|e| anyhow::anyhow!("Failed to type Enter: {}", e))?;
            }
            '\t' => {
                self.enigo.key(Key::Tab, Direction::Click)
                    .map_err(|e| anyhow::anyhow!("Failed to type Tab: {}", e))?;
            }
            ' ' => {
                self.enigo.key(Key::Space, Direction::Click)
                    .map_err(|e| anyhow::anyhow!("Failed to type Space: {}", e))?;
            }
            // Regular characters
            _ => {
                self.enigo.text(&c.to_string())
                    .map_err(|e| anyhow::anyhow!("Failed to type character '{}': {}", c, e))?;
            }
        }
        Ok(())
    }

    pub async fn inject_with_formatting(&mut self, text: &str) -> Result<()> {
        if text.is_empty() {
            return Ok(());
        }

        info!("📝 Injecting formatted text: \"{}\"", text);

        // Add a small delay
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Clean up the text (remove extra whitespace, fix punctuation)
        let cleaned_text = self.clean_text(text);

        // Type the cleaned text
        self.inject_text_fast(&cleaned_text).await?;

        Ok(())
    }

    fn clean_text(&self, text: &str) -> String {
        text.trim()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .replace(" ,", ",")
            .replace(" .", ".")
            .replace(" !", "!")
            .replace(" ?", "?")
            .replace(" ;", ";")
            .replace(" :", ":")
    }

    pub async fn clear_and_inject(&mut self, text: &str) -> Result<()> {
        // Select all text (Ctrl+A)
        self.enigo.key(Key::Control, Direction::Press)
            .map_err(|e| anyhow::anyhow!("Failed to press Ctrl: {}", e))?;
        self.enigo.key(Key::Unicode('a'), Direction::Click)
            .map_err(|e| anyhow::anyhow!("Failed to press A: {}", e))?;
        self.enigo.key(Key::Control, Direction::Release)
            .map_err(|e| anyhow::anyhow!("Failed to release Ctrl: {}", e))?;

        tokio::time::sleep(Duration::from_millis(10)).await;

        // Delete selected text and inject new text
        self.inject_text_fast(text).await?;

        Ok(())
    }
}