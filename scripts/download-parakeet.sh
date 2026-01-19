#!/bin/bash
# Download Parakeet TDT 0.6B v2 model and Silero VAD for TomChat
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
MODELS_DIR="$PROJECT_DIR/models"

echo "Creating models directory..."
mkdir -p "$MODELS_DIR"
cd "$MODELS_DIR"

# Download Parakeet TDT 0.6B v2 (INT8 quantized)
PARAKEET_MODEL="sherpa-onnx-nemo-parakeet-tdt-0.6b-v2-int8"
PARAKEET_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/${PARAKEET_MODEL}.tar.bz2"

if [ ! -d "$PARAKEET_MODEL" ]; then
    echo "Downloading Parakeet TDT 0.6B v2 model (~180MB)..."
    wget -q --show-progress "$PARAKEET_URL" -O "${PARAKEET_MODEL}.tar.bz2"
    echo "Extracting model..."
    tar xjf "${PARAKEET_MODEL}.tar.bz2"
    rm "${PARAKEET_MODEL}.tar.bz2"
    echo "Parakeet model ready!"
else
    echo "Parakeet model already exists, skipping download."
fi

# Download Silero VAD model
SILERO_VAD="silero_vad.onnx"
SILERO_URL="https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/silero_vad.onnx"

if [ ! -f "$SILERO_VAD" ]; then
    echo "Downloading Silero VAD model..."
    wget -q --show-progress "$SILERO_URL" -O "$SILERO_VAD"
    echo "Silero VAD model ready!"
else
    echo "Silero VAD model already exists, skipping download."
fi

echo ""
echo "=== All models downloaded successfully! ==="
echo ""
echo "Model files:"
ls -lh "$MODELS_DIR"
echo ""
echo "You can now build and run TomChat:"
echo "  cargo build --release"
echo "  ./target/release/tomchat"
