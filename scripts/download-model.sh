#!/bin/bash
set -e

# TomChat Model Download Script
# Downloads Whisper models for local speech-to-text

MODELS_DIR="./models"
BASE_URL="https://huggingface.co/ggerganov/whisper.cpp/resolve/main"

# Create models directory
mkdir -p "$MODELS_DIR"

echo "🐕 TomChat Model Downloader"
echo "=========================="
echo ""

# Available models with sizes and descriptions
declare -A MODELS=(
    ["tiny"]="39M - English-only, fastest, lower accuracy"
    ["base"]="142M - Multilingual, good balance of speed/accuracy" 
    ["small"]="466M - Multilingual, better accuracy, slower"
    ["medium"]="1.5G - Multilingual, high accuracy, much slower"
    ["large-v3"]="2.9G - Multilingual, highest accuracy, very slow"
)

echo "Available Whisper models:"
echo ""
for model in tiny base small medium large-v3; do
    echo "  $model: ${MODELS[$model]}"
done
echo ""

# Get user choice or use argument
if [ $# -eq 0 ]; then
    read -p "Which model would you like to download? [base]: " MODEL_CHOICE
    MODEL_CHOICE=${MODEL_CHOICE:-base}
else
    MODEL_CHOICE=$1
fi

# Validate model choice
if [[ ! "${!MODELS[@]}" =~ "$MODEL_CHOICE" ]]; then
    echo "❌ Invalid model choice: $MODEL_CHOICE"
    echo "Available: ${!MODELS[@]}"
    exit 1
fi

# Set filenames based on model
case $MODEL_CHOICE in
    "large-v3")
        FILENAME="ggml-large-v3.bin"
        ;;
    *)
        FILENAME="ggml-${MODEL_CHOICE}.bin"
        ;;
esac

MODEL_PATH="$MODELS_DIR/$FILENAME"

# Check if model already exists
if [ -f "$MODEL_PATH" ]; then
    echo "✅ Model already exists: $MODEL_PATH"
    echo "   Size: $(du -h "$MODEL_PATH" | cut -f1)"
    read -p "Re-download anyway? [y/N]: " REDOWNLOAD
    if [[ ! "$REDOWNLOAD" =~ ^[Yy]$ ]]; then
        echo "Using existing model."
        exit 0
    fi
fi

echo ""
echo "📥 Downloading $MODEL_CHOICE model (${MODELS[$MODEL_CHOICE]})..."
echo "   URL: $BASE_URL/$FILENAME"
echo "   Destination: $MODEL_PATH"
echo ""

# Download with progress bar
if command -v wget >/dev/null 2>&1; then
    wget --progress=bar:force:noscroll -O "$MODEL_PATH" "$BASE_URL/$FILENAME"
elif command -v curl >/dev/null 2>&1; then
    curl -L --progress-bar -o "$MODEL_PATH" "$BASE_URL/$FILENAME"
else
    echo "❌ Neither wget nor curl found. Please install one of them."
    exit 1
fi

echo ""
echo "✅ Model downloaded successfully!"
echo "   File: $MODEL_PATH"
echo "   Size: $(du -h "$MODEL_PATH" | cut -f1)"
echo ""
echo "🔧 Update your config.toml:"
echo "   [whisper]"
echo "   model_path = \"$(pwd)/$MODEL_PATH\""
echo ""
echo "🚀 Ready to run TomChat!"