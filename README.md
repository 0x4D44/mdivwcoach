# Interview Coach v2.0

AI-powered interview practice tool for software engineers. Practice C++, OO design, networking, and AI tools topics at your target IC level.

⭐ **NEW in v2.0**: Enhanced AI feedback with real-time progress, token usage tracking, and example answer generation! See [FEATURES.md](FEATURES.md) for detailed overview.

## Features

- 🎯 **IC Level-Focused**: Questions tailored to IC3 through IC8 expectations
- 🧠 **AI-Powered**: Dynamic question generation and intelligent grading
- 📊 **Multi-Provider**: Supports Claude, OpenAI, and Gemini APIs
- 📈 **Progress Tracking**: SQLite database tracks your improvement over time
- ⚡ **Fast & Portable**: Single Rust binary, no dependencies
- 📝 **Flexible Input**: Choose between terminal multi-line editor or external editor
- 🔍 **Detailed Feedback**: Real-time AI progress updates with token usage statistics
- 🌟 **Example Answers**: Generate high-quality example responses based on improvement suggestions

## Setup

1. **Install Rust** (if not already installed):
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **Clone and build**:
   ```bash
   cd G:\ClaudeMCP\interview_coach
   cargo build --release
   ```

3. **Set API keys** (at least one required):
   ```bash
   # Windows
   set CLAUDE_API_KEY=your_claude_key_here
   set OPENAI_API_KEY=your_openai_key_here
   set GEMINI_API_KEY=your_gemini_key_here
   set PREFERRED_AI_PROVIDER=claude
   
   # Linux/Mac
   export CLAUDE_API_KEY=your_claude_key_here
   export OPENAI_API_KEY=your_openai_key_here
   export GEMINI_API_KEY=your_gemini_key_here
   export PREFERRED_AI_PROVIDER=claude
   ```

## Usage

### Interactive Mode (Recommended)
```bash
cargo run
```

### Direct Mode
```bash
# Quick principal-level C++ session
cargo run -- --topic cpp --level ic7 --count 3

# Senior networking practice with external editor
cargo run -- --topic networking --level ic5 --count 5 --input editor

# Use rustyline for multi-line terminal editing (default)
cargo run -- --topic ai_tools --level ic7 --input rustyline
```

### Commands
```bash
# Interactive setup
cargo run interactive

# Review recent sessions
cargo run review --limit 5

# Get improvement suggestions
cargo run improve
```

## Topics Available

- **cpp**: C++ language, memory management, performance
- **oo_design**: Object-oriented design patterns and principles
- **networking**: IP networking, protocols, distributed systems
- **ai_tools**: AI development tools, ML pipelines, LLM integration

## IC Levels

- **IC3**: Mid-level fundamentals
- **IC5**: Senior system design
- **IC6**: Staff cross-system architecture
- **IC7**: Principal domain expertise ⭐ (Your level)
- **IC8**: Distinguished industry leadership

## Input Methods

**Rustyline (Default)**
- Multi-line terminal editing with line numbers
- Arrow key navigation, copy/paste support
- Type 'END' on new line or Ctrl+D to finish
- Great for quick answers and code snippets

**External Editor**
- Opens your default text editor (VS Code, Notepad++, vim, etc.)
- Full editing capabilities with syntax highlighting
- Save and close editor to submit answer
- Perfect for longer, complex responses

Choose your preferred method at startup or use `--input` flag for CLI mode.

## Example Session

```
🚀 Welcome to Interview Coach!
Let's set up your practice session.

Select a topic to practice: cpp
Select your IC level: ic7
How would you like to input your answers?
> Rustyline (Multi-line terminal editing)
  External Editor (Opens your default editor)
How many questions? (1-10): 3

🎯 Starting cpp interview session for ic7 level
You'll be asked 3 questions. Take your time to think through each answer.

📝 Question 1 of 3

Your team is debating whether to migrate a performance-critical 
financial trading system from raw pointers to smart pointers. 
The current system processes 100k trades/second with microsecond 
latency requirements. Walk through your technical decision framework.

💡 This tests your ability to balance technical best practices 
with business constraints - typical IC7 principal-level thinking.

⏱️  Suggested time: 10 minutes

📝 Enter your answer (press Ctrl+D or type 'END' on a new line when finished):
    💡 Use Enter for new lines, arrow keys to navigate

 1> I would approach this decision systematically by analyzing...
 2> the performance implications, team readiness, and migration...
 3> strategy. First, I'd benchmark the current system to establish...
 4> END

📊 Grading Results:
   Technical Accuracy: 8/10
   Problem Solving:    9/10
   Communication:      7/10
   Leadership:         8/10
   Overall Score:      8/10

💬 Feedback:
Strong systematic approach and good awareness of performance 
implications. Consider discussing team alignment strategies 
and migration timeline planning.

🚀 Improvement Suggestions:
   • Include specific metrics for measuring migration success
   • Address stakeholder communication during the transition
```

## File Structure

- `interview_sessions.db`: SQLite database with your progress
- `Cargo.toml`: Rust dependencies
- `src/main.rs`: Complete application (single file)

## Troubleshooting

### API Response Issues
If you see "Failed to parse question response" errors:
- **Check API keys**: Ensure your API key is correctly set
- **Check internet connection**: API calls require network access
- **Provider issues**: Try switching providers with `PREFERRED_AI_PROVIDER`
- **Fallback mode**: The tool will use built-in questions if API parsing fails

### Common Errors
- **"Claude API key not set"**: Set `CLAUDE_API_KEY` environment variable
- **JSON Parse Error**: The tool will show debug output and use fallback questions
- **Build Errors**: Ensure you have the latest Rust version (`rustup update`)
- **Database Issues**: Delete `interview_sessions.db` to reset

### Troubleshooting Input Issues

**Rustyline Mode**
- Use arrow keys to navigate within lines
- Press Enter for new lines
- Type 'END' on a new line to finish your answer
- Use Ctrl+D as alternative to finish
- Copy/paste works with standard terminal shortcuts

**External Editor Mode**
- If no editor opens, set your default editor:
  - Windows: `set EDITOR=notepad` or `set EDITOR=code`
  - Linux/Mac: `export EDITOR=vim` or `export EDITOR=code`
- Save the file and close the editor to submit
- Empty files will be treated as empty answers

## Environment Variables

```bash
# Required (at least one)
CLAUDE_API_KEY=your_claude_key
OPENAI_API_KEY=your_openai_key  
GEMINI_API_KEY=your_gemini_key

# Optional
PREFERRED_AI_PROVIDER=claude   # claude, openai, or gemini
```

## Contributing

This is a personal practice tool, but feel free to fork and customize for your needs!
