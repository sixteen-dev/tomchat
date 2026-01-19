#!/bin/bash
# Setup script for Gemma 3 model direct download

set -e

echo "🧠 TomChat Gemma 3 Setup Script"
echo "================================"

# Install Python requests if not available
if ! python3 -c "import requests" &> /dev/null; then
    echo "📦 Installing Python requests..."
    pip install --user requests
fi

# Create models directory
mkdir -p ./models
MODEL_PATH="./models/gemma-3-1b-it"
mkdir -p "$MODEL_PATH"

# Function to download file with progress
download_file() {
    local url=$1
    local output=$2
    local desc=$3
    
    echo "📥 Downloading $desc..."
    
    # Use curl with progress bar if available, otherwise wget
    if command -v curl &> /dev/null; then
        curl -L --progress-bar "$url" -o "$output"
    elif command -v wget &> /dev/null; then
        wget --progress=bar "$url" -O "$output"
    else
        echo "❌ Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Download required files directly from HuggingFace (no auth needed for some files)
echo ""
echo "📥 Downloading Gemma 3 1B model files..."
echo "This may take several minutes depending on your connection."

# Check for HuggingFace token
HF_TOKEN=${HF_TOKEN:-}
if [ -z "$HF_TOKEN" ]; then
    echo ""
    echo "🔐 HuggingFace Token Required"
    echo "============================="
    echo ""
    echo "Please set your HuggingFace token:"
    echo "  export HF_TOKEN=your_token_here"
    echo ""
    echo "Then re-run this script."
    echo ""
    echo "To get a token:"
    echo "1. Go to: https://huggingface.co/settings/tokens"
    echo "2. Create a token with 'Read' permissions"
    echo "3. Accept license: https://huggingface.co/google/gemma-3-1b-it"
    exit 1
fi

# Function to download with authentication
download_with_auth() {
    local url=$1
    local output=$2
    local desc=$3
    
    echo "📥 Downloading $desc..."
    
    if command -v curl &> /dev/null; then
        curl -L --progress-bar -H "Authorization: Bearer $HF_TOKEN" "$url" -o "$output"
    elif command -v wget &> /dev/null; then
        wget --progress=bar --header="Authorization: Bearer $HF_TOKEN" "$url" -O "$output"
    else
        echo "❌ Neither curl nor wget found. Please install one of them."
        exit 1
    fi
}

# Download all required files with authentication
echo "📥 Downloading Gemma 3 1B model files with authentication..."

if [ ! -f "$MODEL_PATH/config.json" ]; then
    download_with_auth "https://huggingface.co/google/gemma-3-1b-it/resolve/main/config.json" "$MODEL_PATH/config.json" "config.json"
fi

if [ ! -f "$MODEL_PATH/tokenizer.json" ]; then
    download_with_auth "https://huggingface.co/google/gemma-3-1b-it/resolve/main/tokenizer.json" "$MODEL_PATH/tokenizer.json" "tokenizer.json"
fi

if [ ! -f "$MODEL_PATH/tokenizer_config.json" ]; then
    download_with_auth "https://huggingface.co/google/gemma-3-1b-it/resolve/main/tokenizer_config.json" "$MODEL_PATH/tokenizer_config.json" "tokenizer_config.json"
fi

if [ ! -f "$MODEL_PATH/model.safetensors" ]; then
    echo ""
    echo "📥 Downloading model weights (~1.2GB)..."
    echo "This will take several minutes..."
    download_with_auth "https://huggingface.co/google/gemma-3-1b-it/resolve/main/model.safetensors" "$MODEL_PATH/model.safetensors" "model.safetensors"
fi

# Verify files exist
REQUIRED_FILES=("config.json" "tokenizer.json" "model.safetensors")
echo ""
echo "🔍 Verifying model files..."
for file in "${REQUIRED_FILES[@]}"; do
    if [ -f "$MODEL_PATH/$file" ]; then
        echo "✅ $file"
    else
        echo "❌ $file - Missing!"
        exit 1
    fi
done

echo ""
echo "🎉 Setup complete! You can now run TomChat with text refinement enabled."
echo ""
echo "To test:"
echo "  ./target/release/tomchat"
echo ""
echo "To run with debug logging:"
echo "  RUST_LOG=tomchat::text_refinement=debug,tomchat=info ./target/release/tomchat"