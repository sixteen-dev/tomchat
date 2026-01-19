# TomChat

> Fast, local speech-to-text with a global hotkey (CLI version)

TomChat is a lightweight Linux CLI application that converts speech to text and types it into any application. Press a hotkey, speak, and your words appear. All processing happens locally - no cloud services required.

## Features

- **Local Speech Recognition** - NVIDIA Parakeet TDT 0.6B with ~6% WER (better than Whisper)
- **Voice Activity Detection** - Silero VAD auto-stops recording after silence
- **Global Hotkey** - Works in any application (Caps Lock by default)
- **Text Refinement** - Optional Ollama integration for fixing transcription errors
- **Lightweight** - Pure Rust CLI, minimal footprint

## Requirements

- **OS**: Linux (Ubuntu 22.04+ recommended)
- **RAM**: 4GB minimum
- **Disk**: ~200MB for models (INT8)

## Quick Start

### 1. Install Dependencies

```bash
sudo apt update
sudo apt install -y \
    build-essential \
    libasound2-dev \
    libssl-dev \
    pkg-config \
    wget

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
```

### 2. Clone and Build

```bash
git clone https://github.com/sixteen-dev/tomchat.git
cd tomchat
cargo build --release
```

### 3. Download Models

```bash
# Run the download script
./scripts/download-parakeet.sh
```

This downloads:
- **Parakeet TDT 0.6B v2 (INT8)** - ~180MB speech recognition model
- **Silero VAD** - ~2MB voice activity detection model

After downloading, your `models/` directory should contain:
```
models/
├── sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8/
│   ├── encoder.int8.onnx
│   ├── decoder.int8.onnx
│   ├── joiner.int8.onnx
│   └── tokens.txt
└── silero_vad.onnx
```

### 4. Run

```bash
# Use the wrapper script (sets library path automatically)
./tomchat

# Or run directly with library path
LD_LIBRARY_PATH=./target/release ./target/release/tomchat
```

### 5. Use

1. Press **Caps Lock** to start recording
2. Speak into your microphone
3. Wait for auto-stop (1.5s silence) or press **Caps Lock** again
4. Text appears in the focused application

## Configuration

Edit `config.toml` to customize:

```toml
[hotkey]
combination = "caps"  # Options: "caps", "ctrl+shift+space", "f24", etc.

[vad]
model_path = "./models/silero_vad.onnx"
sensitivity = "Normal"  # Low, Normal, High, VeryHigh
timeout_ms = 1500       # Auto-stop after this much silence
auto_stop = true        # Set false for manual stop only

[speech]
model_dir = "./models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"
language = "en"

[text_refinement]
enabled = false         # Enable for Ollama-based text cleanup
model_name = "gemma3:1b"
ollama_url = "http://localhost:11434"
```

### Environment Variables

```bash
export TOMCHAT_MODEL_DIR="/path/to/models"
export TOMCHAT_HOTKEY="ctrl+alt+c"
```

## Text Refinement (Optional)

TomChat can use Ollama to fix transcription errors:

```bash
# Install Ollama
curl -fsSL https://ollama.com/install.sh | sh

# Pull a small model
ollama pull gemma3:1b

# Enable in config.toml
[text_refinement]
enabled = true
```

This fixes issues like:
- "cooper nettys" → "Kubernetes"
- Missing punctuation and capitalization

## Manual Model Download

If the script doesn't work, download manually:

```bash
mkdir -p models
cd models

# Parakeet model
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2
tar xjf sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2
rm sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8.tar.bz2

# Silero VAD
wget https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx
```

## Troubleshooting

### Hotkey Not Working

```bash
# Try a different hotkey in config.toml
combination = "ctrl+shift+space"

# Or use a function key
combination = "f24"
```

### Audio Issues

```bash
# List audio devices
arecord -l

# Test recording
arecord -d 3 test.wav && aplay test.wav
```

### Library Not Found

```bash
# Always use the wrapper script
./tomchat

# Or set LD_LIBRARY_PATH manually
LD_LIBRARY_PATH=./target/release ./target/release/tomchat
```

### Debug Mode

```bash
RUST_LOG=debug ./tomchat
```

## Project Structure

```
tomchat/
├── src/
│   ├── main.rs           # Entry point
│   ├── app.rs            # Application logic
│   ├── config.rs         # Configuration handling
│   ├── audio/
│   │   ├── mod.rs        # Audio capture
│   │   └── vad.rs        # Voice activity detection
│   ├── speech/
│   │   └── transcriber.rs # Parakeet transcription
│   └── text_refinement/  # Optional Ollama integration
├── scripts/
│   └── download-parakeet.sh
├── config.toml           # Default configuration
├── Cargo.toml
└── README.md
```

## Performance

| Metric | Value |
|--------|-------|
| Startup | ~2-3s |
| Transcription | 0.5-2s |
| Memory | ~400MB |
| Model Size | ~200MB |

## Tech Stack

- **[sherpa-rs](https://github.com/thewh1teagle/sherpa-rs)** - Rust bindings for sherpa-onnx
- **[cpal](https://github.com/RustAudio/cpal)** - Cross-platform audio
- **[global-hotkey](https://github.com/tauri-apps/global-hotkey)** - System-wide hotkeys
- **[enigo](https://github.com/enigo-rs/enigo)** - Text injection
- **[tokio](https://tokio.rs/)** - Async runtime

## Related

- **[tomchat-app](https://github.com/sixteen-dev/tomchat-app)** - GUI version with Tauri

## License

MIT License - see [LICENSE](LICENSE)

---

**Named after Tommy** - Built with Rust + sherpa-rs
