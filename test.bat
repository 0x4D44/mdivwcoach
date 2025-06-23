@echo off
echo 🧪 Testing Interview Coach
echo =========================
echo.

echo Setting up test environment...
set CLAUDE_API_KEY=test_key
set PREFERRED_AI_PROVIDER=claude

echo.
echo 🔧 Testing build...
cargo build --release --manifest-path G:\ClaudeMCP\interview_coach\Cargo.toml

if %errorlevel% neq 0 (
    echo ❌ Build failed!
    pause
    exit /b 1
)

echo ✅ Build successful!
echo.

echo 📋 To test the application:
echo 1. Set your actual API key: set CLAUDE_API_KEY=your_actual_key
echo 2. Run: cargo run --manifest-path G:\ClaudeMCP\interview_coach\Cargo.toml
echo.

echo 🎯 The application now includes:
echo   - Better JSON parsing with fallback questions
echo   - Debug output if API calls fail
echo   - Manual fallback questions for each topic/level
echo   - More robust error handling
echo.

pause
