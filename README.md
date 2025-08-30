# TomChat 🐕

> Speech-to-Text Hotkey Application - Named after Tommy

TomChat is a lightweight Linux application that converts speech to text using Whisper AI and injects it into any text field via a global hotkey. Built with Rust using professional, battle-tested crates.

## Features

- 🎤 **Local Speech-to-Text** - Uses Whisper AI model locally (no internet required)
- ⌨️ **Global Hotkey Activation** - Works in any application with text input
- 🔇 **Smart Silence Detection** - Auto-stops recording after silence timeout
- 📝 **Automatic Text Injection** - Types transcribed text directly into active application
- 🚀 **High Performance** - Async Rust implementation with professional crates
- 🔧 **Configurable** - TOML-based configuration for all settings

## Architecture

**Professional Crates Used:**
- **Audio**: `cpal` for cross-platform audio capture
- **VAD**: `webrtc-vad` for voice activity detection  
- **Speech**: `whisper-rs` for local Whisper AI transcription
- **Hotkeys**: `global-hotkey` for system-wide key detection
- **Text**: `enigo` for cross-platform text injection
- **Async**: `tokio` for high-performance async runtime

## Prerequisites

### System Dependencies

1. **Whisper Model** - Download automatically:
   ```bash
   # Quick setup - downloads base model (142MB)
   ./scripts/download-model.sh
   
   # Or choose specific model size
   ./scripts/download-model.sh small    # 466MB, better accuracy
   ./scripts/download-model.sh tiny     # 39MB, fastest
   ```

2. **Audio System** - ALSA (usually pre-installed on Linux)

3. **Development Tools** - Rust toolchain with libclang:
   ```bash
   # libclang should already be available at:
   /usr/lib/x86_64-linux-gnu/libclang-14.so
   ```

## Quick Start

1. **Setup**:
   ```bash
   cd /home/sujshe/src/tomchat
   ./scripts/download-model.sh          # Downloads base model
   ```

2. **Build & Run**:
   ```bash
   LIBCLANG_PATH="/usr/lib/x86_64-linux-gnu" cargo build --release
   ./target/release/tomchat
   ```

3. **Test**: Press **Meta+Shift**, speak, and see transcribed text appear!

## Configuration

Edit `config.toml` to customize settings:

```toml
[hotkey]
# Current: Super+Shift (Windows/Super + Shift keys)
combination = "super+shift"

[audio]
sample_rate = 16000      # Whisper-optimized sample rate
channels = 1             # Mono audio
buffer_duration_ms = 64  # Low latency

[vad]
sensitivity = "Normal"   # Voice activity detection: Low, Normal, High, VeryHigh  
timeout_ms = 500        # Stop recording after 500ms of silence

[whisper]
# Download models with: ./scripts/download-model.sh
model_path = "./models/ggml-base.bin"  # Automatically downloaded
language = "en"
translate = false

[text]
typing_delay_ms = 1     # Delay between keystrokes
```

### Environment Variables

Override config with environment variables:

```bash
# Use different model
export TOMCHAT_MODEL_PATH="/path/to/your/model.bin"

# Use different hotkey  
export TOMCHAT_HOTKEY="ctrl+alt+c"

# Run with overrides
./target/release/tomchat
```

### Model Management

**Available Models:**
- `tiny` (39MB) - Fastest, English-only, lower accuracy
- `base` (142MB) - **Recommended** - Good balance of speed/accuracy
- `small` (466MB) - Better accuracy, slower
- `medium` (1.5GB) - High accuracy, much slower
- `large-v3` (2.9GB) - Highest accuracy, very slow

**Download Options:**
```bash
./scripts/download-model.sh base      # Recommended
./scripts/download-model.sh small     # Better quality
./scripts/download-model.sh tiny      # Fastest
```

## Usage

1. **Start TomChat**:
   ```bash
   cd /home/sujshe/src/tomchat
   ./target/release/tomchat
   ```

2. **You should see**:
   ```
   🐕 TomChat - Speech-to-Text Hotkey Application
      Named after Tommy
      Powered by Rust + Professional Crates
   ✅ Configuration loaded successfully
   🚀 TomChat is ready! Press meta+shift to start recording.
   Press Ctrl+C to exit.
   ```

3. **Test the Feature**:
   - Open any text editor (VS Code, terminal, browser text field, etc.)
   - Press **Meta+Shift** (Windows/Super key + Shift)
   - Speak clearly into your microphone
   - TomChat will automatically stop after silence and inject the transcribed text

## Testing

### Quick Test

1. **Terminal Test**:
   ```bash
   # Open a new terminal
   nano test.txt
   # Press Meta+Shift, say "Hello world this is a test"
   # Should see text appear in nano
   ```

2. **Browser Test**:
   - Open browser, go to any text field
   - Press Meta+Shift, speak your message
   - Text should appear in the field

3. **VS Code Test**:
   - Open VS Code with a file
   - Press Meta+Shift, dictate some code or comments
   - Should see transcribed text

### Troubleshooting

**Audio Issues**:
```bash
# Check audio devices
arecord -l

# Test microphone
arecord -f cd -t wav -d 5 test.wav && aplay test.wav
```

**Hotkey Issues**:
- Try different key combinations in config.toml:
  - `"ctrl+alt+c"` (safe alternative)
  - `"super+shift"` (Windows/Super key + Shift - current default)  
  - `"ctrl+shift+space"` (Ctrl + Shift + Space)
  - `"f24"` (if Copilot key sends F24)

**Supported Keys:**
- **Modifiers**: `ctrl`, `shift`, `alt`, `super`/`win`/`meta`
- **Keys**: `space`, `a-z`, `f1-f24`, etc.

**Model Issues**:
- Verify model exists: `ls -la /home/sujshe/src/whisper-hotkey-cpp/models/ggml-small.bin`
- Download if missing from [Whisper models](https://huggingface.co/ggerganov/whisper.cpp)

**Permission Issues**:
- Ensure user is in `audio` group: `groups $USER`
- Add if needed: `sudo usermod -a -G audio $USER`

## Development

**Debug Build**:
```bash
LIBCLANG_PATH="/usr/lib/x86_64-linux-gnu" cargo build
./target/debug/tomchat
```

**View Logs**:
```bash
RUST_LOG=debug ./target/release/tomchat
```

**Hot Reload Development**:
```bash
cargo watch -x 'build' -x 'run'
```

## Performance

- **Startup**: ~1-2 seconds (loading Whisper model)
- **Activation**: Instant hotkey detection
- **Transcription**: ~1-3 seconds depending on speech length
- **Memory**: ~100-200MB (mainly Whisper model)
- **CPU**: Low idle, moderate during transcription

## Integration with Claude Code

TomChat is designed to work seamlessly with Claude Code:

1. **Start TomChat** in background
2. **Open Claude Code** terminal or any text field  
3. **Press Meta+Shift** and speak your query/code
4. **Continue conversation** with transcribed text

Perfect for hands-free coding sessions!

---

**Named after Tommy 🐕** - Built with Rust + Professional Crates