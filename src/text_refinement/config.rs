use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRefinementConfig {
    pub enabled: bool,
    pub model_name: String,
    pub ollama_url: String,
    // Keep some legacy fields for backward compatibility (unused with Ollama)
    #[serde(default)]
    pub device: String,
    #[serde(default)]
    pub cpu_threads: u32,
    #[serde(default)]
    pub quantization: String,
    #[serde(default)]
    pub batch_size: u32,
    // Core Ollama configuration
    pub prompt_template: String,
    pub max_tokens: u32,
    pub temperature: f32,
    pub timeout_ms: u64,
    #[serde(default)]
    pub max_retries: u32,
    pub fallback_on_timeout: bool,
}

impl Default for TextRefinementConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            model_name: "gemma3:1b".to_string(),
            ollama_url: "http://localhost:11434".to_string(),
            device: "cpu".to_string(), // Legacy field
            cpu_threads: 0, // Legacy field
            quantization: "int4".to_string(), // Legacy field
            batch_size: 1, // Legacy field
            prompt_template: r#"Fix transcription errors in this speech-to-text from a developer/technical context:

Common fixes needed:
• "cooper tease" → "Kubernetes" 
• "docker" → "Docker"
• "get hub" → "GitHub"
• "pie thon" → "Python" 
• "A-P-I" → "API"
• "S-S-H" → "SSH"
• "react J-S" → "React.js"
• Technical acronyms spelled out → proper form

Original: "{text}"
Corrected:"#.to_string(),
            max_tokens: 150,
            temperature: 0.1,
            timeout_ms: 8000, // 8 seconds for Ollama
            max_retries: 1,
            fallback_on_timeout: true,
        }
    }
}