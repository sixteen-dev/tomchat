#!/bin/bash
set -e

echo "🦙 Setting up Llama-3.2-1B for TomChat Text Refinement"
echo "======================================================"

# Check for HuggingFace token
if [ -z "$HF_TOKEN" ]; then
    echo ""
    echo "⚠️  No HuggingFace token found in environment variable HF_TOKEN"
    echo ""
    echo "To get a token:"
    echo "1. Go to: https://huggingface.co/settings/tokens"
    echo "2. Create a token with 'Read' permissions"
    echo "3. Accept license: https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct"
    echo ""
    echo "Usage: HF_TOKEN=your_token_here ./scripts/setup-llama.sh"
    exit 1
fi

# Create models directory
mkdir -p ./models
MODEL_PATH="./models/llama-3.2-1b-instruct"
mkdir -p "$MODEL_PATH"

# Function to download file with progress
download_file() {
    local url="$1"
    local output="$2"
    local description="$3"
    
    if [ ! -f "$output" ]; then
        echo "📥 Downloading $description..."
        curl -L --progress-bar -o "$output" \
             -H "Authorization: Bearer $HF_TOKEN" \
             "$url"
        echo "✅ Downloaded $description"
    else
        echo "✅ $description already exists"
    fi
}

# Function to download with authentication
download_with_auth() {
    local url="$1"
    local output="$2"
    local description="$3"
    
    if curl -L -H "Authorization: Bearer $HF_TOKEN" -o "$output" "$url"; then
        echo "✅ Downloaded $description"
    else
        echo "❌ Failed to download $description"
        exit 1
    fi
}

# Download all required files with authentication
echo "📥 Downloading Llama-3.2-1B model files with authentication..."

if [ ! -f "$MODEL_PATH/config.json" ]; then
    download_with_auth "https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct/resolve/main/config.json" "$MODEL_PATH/config.json" "config.json"
fi

if [ ! -f "$MODEL_PATH/tokenizer.json" ]; then
    download_with_auth "https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct/resolve/main/tokenizer.json" "$MODEL_PATH/tokenizer.json" "tokenizer.json"
fi

if [ ! -f "$MODEL_PATH/tokenizer_config.json" ]; then
    download_with_auth "https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct/resolve/main/tokenizer_config.json" "$MODEL_PATH/tokenizer_config.json" "tokenizer_config.json"
fi

if [ ! -f "$MODEL_PATH/model.safetensors" ]; then
    echo "📥 Downloading model weights (~1.2GB)..."
    download_with_auth "https://huggingface.co/meta-llama/Llama-3.2-1B-Instruct/resolve/main/model.safetensors" "$MODEL_PATH/model.safetensors" "model.safetensors"
fi

echo ""
echo "🎉 Llama-3.2-1B setup complete!"
echo ""
echo "📁 Model files saved to: $MODEL_PATH"
echo "🚀 You can now run: cargo run --release"
echo ""
echo "The model will be used for text refinement to fix:"
echo "• Technical terms that Whisper mishears due to accent"
echo "• Add proper punctuation and grammar"
echo "• Correct transcription errors in developer context"