#!/bin/bash

echo "🚀 Interview Coach Setup"
echo "========================"
echo

echo "Step 1: Building the application..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed! Make sure Rust is installed."
    echo "   Visit: https://rustup.rs/"
    exit 1
fi

echo "✅ Build successful!"
echo

echo "Step 2: Setting up API keys..."
echo
echo "You need at least one API key. Add to your ~/.bashrc or ~/.zshrc:"
echo
echo "For Claude (recommended):"
echo "  export CLAUDE_API_KEY=your_key_here"
echo
echo "For OpenAI:"
echo "  export OPENAI_API_KEY=your_key_here"
echo
echo "For Gemini:"
echo "  export GEMINI_API_KEY=your_key_here"
echo
echo "Set preferred provider:"
echo "  export PREFERRED_AI_PROVIDER=claude"
echo

echo "🎯 Ready to start! Run:"
echo "  cargo run"
echo
echo "Or for quick practice:"
echo "  cargo run -- --topic cpp --level ic7 --count 3"
echo
