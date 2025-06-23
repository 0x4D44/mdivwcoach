@echo off
echo 🚀 Interview Coach Setup for Windows
echo ====================================

echo.
echo Step 1: Building the application...
cargo build --release

if %errorlevel% neq 0 (
    echo ❌ Build failed! Make sure Rust is installed.
    echo    Visit: https://rustup.rs/
    pause
    exit /b 1
)

echo ✅ Build successful!
echo.

echo Step 2: Setting up API keys...
echo.
echo You need at least one API key. Set them as environment variables:
echo.
echo For Claude (recommended):
echo   set CLAUDE_API_KEY=your_key_here
echo.
echo For OpenAI:
echo   set OPENAI_API_KEY=your_key_here
echo.
echo For Gemini:
echo   set GEMINI_API_KEY=your_key_here
echo.
echo Set preferred provider:
echo   set PREFERRED_AI_PROVIDER=claude
echo.

echo 🎯 Ready to start! Run:
echo   cargo run
echo.
echo Or for quick practice:
echo   cargo run -- --topic cpp --level ic7 --count 3
echo.

pause
