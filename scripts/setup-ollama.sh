#!/bin/bash
set -e

echo "🦙 Setting up Ollama for TomChat Text Refinement"
echo "==============================================="

# Check if Ollama is installed
if ! command -v ollama &> /dev/null; then
    echo "❌ Ollama is not installed"
    echo ""
    echo "Please install Ollama first:"
    echo "curl -fsSL https://ollama.com/install.sh | sh"
    echo ""
    echo "Or visit: https://ollama.com/download"
    exit 1
fi

echo "✅ Ollama is installed"

# Check if Ollama service is running
if ! pgrep -f "ollama serve" > /dev/null; then
    echo "⚠️  Ollama service is not running"
    echo "Starting Ollama service..."
    ollama serve &
    OLLAMA_PID=$!
    sleep 3
    echo "✅ Ollama service started (PID: $OLLAMA_PID)"
else
    echo "✅ Ollama service is already running"
fi

# Pull Gemma 3 1B model
echo ""
echo "📥 Pulling Gemma 3 1B model..."
echo "This may take several minutes depending on your connection."

if ollama pull gemma3:1b; then
    echo "✅ Gemma 3 1B model downloaded successfully"
else
    echo "❌ Failed to download Gemma 3 1B model"
    echo ""
    echo "Trying Gemma 2 1B as fallback..."
    if ollama pull gemma2:1b; then
        echo "✅ Gemma 2 1B model downloaded successfully"
        echo "⚠️  Note: Update config.toml to use 'gemma2:1b' instead of 'gemma3:1b'"
    else
        echo "❌ Failed to download any Gemma model"
        exit 1
    fi
fi

# Test the model
echo ""
echo "🧪 Testing model..."
if echo "Test" | ollama run gemma3:1b > /dev/null 2>&1; then
    echo "✅ Gemma 3 1B model is working correctly"
elif echo "Test" | ollama run gemma2:1b > /dev/null 2>&1; then
    echo "✅ Gemma 2 1B model is working correctly"
else
    echo "❌ Model test failed"
    exit 1
fi

echo ""
echo "🎉 Ollama setup complete!"
echo ""
echo "🚀 You can now run: cargo run --release"
echo ""
echo "The model will be used for text refinement to fix:"
echo "• Technical terms that Whisper mishears due to accent"
echo "• Add proper punctuation and grammar"
echo "• Correct transcription errors in developer context"
echo ""
echo "📝 Available models:"
ollama list