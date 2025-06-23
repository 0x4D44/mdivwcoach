@echo off
echo 🧪 Testing Both Input Methods
echo ===============================
echo.

echo 🔧 Building latest version...
cargo build --release --manifest-path G:\ClaudeMCP\interview_coach\Cargo.toml

if %errorlevel% neq 0 (
    echo ❌ Build failed!
    pause
    exit /b 1
)

echo ✅ Build successful!
echo.

echo 📋 Available input methods:
echo.
echo 1. Rustyline (Default)
echo    - Multi-line terminal editing
echo    - Line numbers and navigation
echo    - Type 'END' to finish or Ctrl+D
echo.
echo 2. External Editor
echo    - Opens your default text editor
echo    - Full editing capabilities
echo    - Save and close to submit
echo.

echo 🎯 Test commands:
echo.
echo Interactive mode (choose input method):
echo   cargo run --manifest-path G:\ClaudeMCP\interview_coach\Cargo.toml
echo.
echo Direct with rustyline:
echo   cargo run --manifest-path G:\ClaudeMCP\interview_coach\Cargo.toml -- --topic cpp --level ic7 --input rustyline
echo.
echo Direct with external editor:
echo   cargo run --manifest-path G:\ClaudeMCP\interview_coach\Cargo.toml -- --topic cpp --level ic7 --input editor
echo.

echo 🔐 Don't forget to set your API key:
echo   set CLAUDE_API_KEY=your_actual_key
echo.

pause
