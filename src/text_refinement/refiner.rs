use anyhow::Result;
use ollama_rs::{Ollama, generation::completion::request::GenerationRequest};
use ollama_rs::models::ModelOptions;
use std::sync::Arc;
use tracing::{debug, info, warn};
use url::Url;

use super::config::TextRefinementConfig;

pub struct TextRefiner {
    ollama: Arc<Ollama>,
    config: TextRefinementConfig,
}

impl TextRefiner {
    pub async fn new(config: TextRefinementConfig) -> Result<Self> {
        if !config.enabled {
            return Err(anyhow::anyhow!("Text refinement is disabled"));
        }

        info!("🦙 Connecting to Ollama for text refinement...");
        info!("Using model: {}", config.model_name);
        info!("Ollama URL: {}", config.ollama_url);
        
        // Create Ollama client
        let url = Url::parse(&config.ollama_url)?;
        let ollama = Ollama::from_url(url);
        
        // Test connection and model availability
        Self::test_connection(&ollama, &config.model_name).await?;
        
        info!("✅ Ollama connection established successfully");

        Ok(Self {
            ollama: Arc::new(ollama),
            config,
        })
    }

    async fn test_connection(ollama: &Ollama, model_name: &str) -> Result<()> {
        // Test basic connection with a simple prompt
        let test_request = GenerationRequest::new(
            model_name.to_string(),
            "Test".to_string(),
        );

        match ollama.generate(test_request).await {
            Ok(_) => {
                info!("✅ Model {} is available and responding", model_name);
                Ok(())
            }
            Err(e) => {
                warn!("❌ Failed to connect to Ollama or load model {}: {}", model_name, e);
                warn!("Make sure:");
                warn!("1. Ollama is running: ollama serve");
                warn!("2. Model is pulled: ollama pull {}", model_name);
                Err(anyhow::anyhow!("Ollama connection failed: {}", e))
            }
        }
    }

    pub async fn refine_text(&self, input_text: &str) -> Result<String> {
        debug!("🔧 Refining text: \"{}\"", input_text);

        // Create prompt from template
        let prompt = self.config.prompt_template.replace("{text}", input_text);
        
        // Calculate dynamic max_tokens based on input length
        // Generally, corrections shouldn't be much longer than original
        let input_word_count = input_text.split_whitespace().count();
        let dynamic_max_tokens = std::cmp::max(
            input_word_count + 50,  // Input + buffer for corrections
            self.config.max_tokens as usize
        ) as i32;

        // Create model options
        let options = ModelOptions::default()
            .temperature(self.config.temperature)
            .top_p(0.9) // Good default for text refinement
            .top_k(40)  // Good default for focused output
            .num_predict(dynamic_max_tokens);

        // Create generation request
        let request = GenerationRequest::new(
            self.config.model_name.clone(),
            prompt,
        ).options(options);

        // Generate refined text with timeout
        let refined_result = tokio::time::timeout(
            std::time::Duration::from_millis(self.config.timeout_ms),
            self.ollama.generate(request),
        ).await;

        match refined_result {
            Ok(Ok(response)) => {
                let refined_text = response.response.trim().to_string();
                info!("✨ Refined: \"{}\" → \"{}\"", input_text, refined_text);
                Ok(refined_text)
            }
            Ok(Err(e)) => {
                warn!("Text refinement failed: {}, using original text", e);
                if self.config.fallback_on_timeout {
                    Ok(input_text.to_string())
                } else {
                    Err(anyhow::anyhow!("Ollama generation failed: {}", e))
                }
            }
            Err(_timeout) => {
                warn!("Text refinement timed out after {}ms, using original text", self.config.timeout_ms);
                if self.config.fallback_on_timeout {
                    Ok(input_text.to_string())
                } else {
                    Err(anyhow::anyhow!("Text refinement timed out"))
                }
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    pub async fn get_model_info(&self) -> String {
        format!("Ollama model: {} at {}", self.config.model_name, self.config.ollama_url)
    }
}