use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::{Parser, Subcommand};
use dialoguer::{Input, Select, Confirm, Editor};
use indicatif::{ProgressBar, ProgressStyle};
use reqwest::Client;
use rusqlite::{params, Connection};
use rustyline::{DefaultEditor, error::ReadlineError};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use uuid::Uuid;

// ===== CONFIGURATION =====

const API_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone)]
struct Config {
    claude_api_key: Option<String>,
    openai_api_key: Option<String>,
    gemini_api_key: Option<String>,
    preferred_provider: String,
}

impl Config {
    fn from_env() -> Self {
        Self {
            claude_api_key: std::env::var("CLAUDE_API_KEY").ok(),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok(),
            gemini_api_key: std::env::var("GEMINI_API_KEY").ok(),
            preferred_provider: std::env::var("PREFERRED_AI_PROVIDER")
                .unwrap_or_else(|_| "claude".to_string()),
        }
    }
}

// ===== DATA STRUCTURES =====

#[derive(Debug, Clone)]
enum InputMethod {
    Rustyline,
    ExternalEditor,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ICLevel {
    level: String,
    focus: String,
    expectation: String,
    time_minutes: u32,
    examples: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuestionResponse {
    question: String,
    difficulty_notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct GradingResponse {
    technical_accuracy: u32,
    problem_solving: u32,
    communication: u32,
    leadership: u32,
    overall_score: u32,
    feedback: String,
    improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ExampleAnswerResponse {
    example_answer: String,
    key_points: Vec<String>,
    why_effective: String,
}

#[derive(Debug, Clone)]
struct APIUsage {
    tokens_used: Option<u32>,
    model_used: String,
    response_time_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionResult {
    id: String,
    topic: String,
    ic_level: String,
    timestamp: DateTime<Utc>,
    questions: Vec<QuestionAnswer>,
    overall_score: f64,
    improvement_areas: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct QuestionAnswer {
    question: String,
    answer: String,
    grading: GradingResponse,
    time_taken_seconds: u64,
}

// ===== IC LEVELS DEFINITION =====

fn get_ic_levels() -> HashMap<String, ICLevel> {
    let mut levels = HashMap::new();
    
    levels.insert("ic3".to_string(), ICLevel {
        level: "IC3 - Mid-level".to_string(),
        focus: "solid fundamentals + basic design".to_string(),
        expectation: "implement well-defined solutions".to_string(),
        time_minutes: 3,
        examples: "implement design patterns, debug performance issues".to_string(),
    });
    
    levels.insert("ic5".to_string(), ICLevel {
        level: "IC5 - Senior".to_string(),
        focus: "system design + trade-off analysis".to_string(),
        expectation: "design components, evaluate alternatives".to_string(),
        time_minutes: 5,
        examples: "design scalable APIs, choose appropriate data structures".to_string(),
    });
    
    levels.insert("ic6".to_string(), ICLevel {
        level: "IC6 - Staff".to_string(),
        focus: "cross-system architecture + technical strategy".to_string(),
        expectation: "influence technical direction, complex problem solving".to_string(),
        time_minutes: 7,
        examples: "design distributed systems, technical debt strategy".to_string(),
    });
    
    levels.insert("ic7".to_string(), ICLevel {
        level: "IC7 - Principal".to_string(),
        focus: "domain expertise + business-technical alignment".to_string(),
        expectation: "drive technical vision, resolve ambiguous problems".to_string(),
        time_minutes: 10,
        examples: "technology selection for business needs, cross-team technical standards".to_string(),
    });
    
    levels.insert("ic8".to_string(), ICLevel {
        level: "IC8 - Distinguished".to_string(),
        focus: "industry-level technical leadership".to_string(),
        expectation: "set technical direction across organization".to_string(),
        time_minutes: 12,
        examples: "architecture for company-wide platforms, technical innovation strategy".to_string(),
    });
    
    levels
}

fn get_topics() -> Vec<&'static str> {
    vec!["cpp", "oo_design", "networking", "ai_tools"]
}

// ===== AI PROVIDER =====

#[derive(Debug)]
struct AIProvider {
    client: Client,
    config: Config,
}

impl AIProvider {
    fn new(config: Config) -> Self {
        let client = Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .expect("Failed to create HTTP client");
            
        Self { client, config }
    }
    
    async fn generate_question(
        &self,
        topic: &str,
        ic_level: &str,
        context: Option<&str>,
    ) -> Result<(QuestionResponse, APIUsage)> {
        let start_time = std::time::Instant::now();
        let ic_levels = get_ic_levels();
        let level_info = ic_levels.get(ic_level)
            .context("Invalid IC level")?;
            
        let prompt = format!(
            "Generate an {} interview question for {} topic.

Level Requirements:
- Focus: {}
- Expectation: {}
- Target time: {} minutes
- Examples: {}

{}

IMPORTANT: You must respond with valid JSON only. No other text before or after.

{{
  \"question\": \"The actual interview question\",
  \"difficulty_notes\": \"Brief explanation of what makes this {} level appropriate\"
}}",
            level_info.level,
            topic,
            level_info.focus,
            level_info.expectation,
            level_info.time_minutes,
            level_info.examples,
            context.map(|c| format!("Context from previous answers: {}", c)).unwrap_or_default(),
            ic_level
        );
        
        let (response, tokens) = self.call_ai_api_with_usage(&prompt).await?;
        let cleaned_response = self.clean_json_response(&response);
        
        // Debug output if parsing fails
        let question: QuestionResponse = serde_json::from_str(&cleaned_response)
            .or_else(|e| {
                eprintln!("JSON Parse Error: {}", e);
                eprintln!("Raw Response: {}", response);
                eprintln!("Cleaned Response: {}", cleaned_response);
                eprintln!("Using fallback question generation...");
                
                // Fallback: create a basic question manually
                Ok::<QuestionResponse, anyhow::Error>(QuestionResponse {
                    question: self.generate_fallback_question(topic, ic_level),
                    difficulty_notes: format!("Fallback question for {} level {}", ic_level, topic),
                })
            })?;
            
        let usage = APIUsage {
            tokens_used: tokens,
            model_used: self.get_model_name(),
            response_time_ms: start_time.elapsed().as_millis() as u64,
        };
            
        Ok((question, usage))
    }
    
    async fn grade_answer(
        &self,
        question: &str,
        answer: &str,
        ic_level: &str,
    ) -> Result<(GradingResponse, APIUsage)> {
        let start_time = std::time::Instant::now();
        let ic_levels = get_ic_levels();
        let level_info = ic_levels.get(ic_level)
            .context("Invalid IC level")?;
            
        let prompt = format!(
            "Grade this {} interview answer on a 1-10 scale:

Question: {}

Answer: {}

Level Context:
- Focus: {}
- Expectation: {}

Score these dimensions (1-10 each):
- Technical accuracy: How correct and complete is the technical content?
- Problem solving: How well does the approach demonstrate systematic thinking?
- Communication: How clearly is the answer structured and explained?
- Leadership: How well does it show strategic thinking and influence (for {})

Provide specific, actionable improvement suggestions.

IMPORTANT: You must respond with valid JSON only. No other text before or after.

{{
  \"technical_accuracy\": 8,
  \"problem_solving\": 7,
  \"communication\": 9,
  \"leadership\": 6,
  \"overall_score\": 7,
  \"feedback\": \"Overall assessment paragraph\",
  \"improvement_suggestions\": [\"specific suggestion 1\", \"specific suggestion 2\"]
}}",
            level_info.level,
            question,
            answer,
            level_info.focus,
            level_info.expectation,
            ic_level
        );
        
        let (response, tokens) = self.call_ai_api_with_usage(&prompt).await?;
        let cleaned_response = self.clean_json_response(&response);
        
        // Debug output if parsing fails
        let grading: GradingResponse = serde_json::from_str(&cleaned_response)
            .or_else(|e| {
                eprintln!("JSON Parse Error: {}", e);
                eprintln!("Raw Response: {}", response);
                eprintln!("Cleaned Response: {}", cleaned_response);
                eprintln!("Using fallback grading...");
                
                // Fallback: create basic grading manually
                Ok::<GradingResponse, anyhow::Error>(GradingResponse {
                    technical_accuracy: 7,
                    problem_solving: 7,
                    communication: 7,
                    leadership: 7,
                    overall_score: 7,
                    feedback: "Unable to parse AI feedback. Please check your API configuration.".to_string(),
                    improvement_suggestions: vec!["Verify API setup".to_string(), "Try again with different provider".to_string()],
                })
            })?;
            
        let usage = APIUsage {
            tokens_used: tokens,
            model_used: self.get_model_name(),
            response_time_ms: start_time.elapsed().as_millis() as u64,
        };
            
        Ok((grading, usage))
    }
    
    fn clean_json_response(&self, response: &str) -> String {
        // Remove control characters and clean up the response
        let cleaned = response
            .chars()
            .filter(|c| !c.is_control() || *c == '\n' || *c == '\r' || *c == '\t')
            .collect::<String>();
            
        // Try to extract JSON from response if it's wrapped in other text
        if let Some(start) = cleaned.find('{') {
            if let Some(end) = cleaned.rfind('}') {
                if end > start {
                    return cleaned[start..=end].to_string();
                }
            }
        }
        
        cleaned
    }
    
    fn generate_fallback_question(&self, topic: &str, ic_level: &str) -> String {
        match (topic, ic_level) {
            ("cpp", "ic7") => "As a principal engineer, how would you approach migrating a legacy C++ codebase from raw pointers to smart pointers while maintaining performance and team productivity?".to_string(),
            ("oo_design", "ic7") => "Design a flexible notification system that can handle multiple types of notifications across different platforms. Walk through your design decisions and trade-offs.".to_string(),
            ("networking", "ic7") => "Your distributed system is experiencing intermittent network partitions. Describe your systematic approach to diagnosing and implementing resilience strategies.".to_string(),
            ("ai_tools", "ic7") => "You need to choose between building an in-house ML platform versus using cloud providers. Walk through your evaluation framework and recommendation process.".to_string(),
            (topic, level) => format!("Describe a challenging technical decision you would make as a {} engineer working with {}. Include your reasoning process and trade-offs.", level, topic),
        }
    }
    
    async fn generate_example_answer(
        &self,
        question: &str,
        ic_level: &str,
        improvement_suggestions: &[String],
    ) -> Result<(ExampleAnswerResponse, APIUsage)> {
        let start_time = std::time::Instant::now();
        let ic_levels = get_ic_levels();
        let level_info = ic_levels.get(ic_level)
            .context("Invalid IC level")?;
            
        let suggestions_text = improvement_suggestions.join(", ");
        
        let prompt = format!(
            "Generate a high-quality example answer for this {} interview question:

Question: {}

Level Context:
- Focus: {}
- Expectation: {}

Key Improvement Areas to Address: {}

Create an exemplary answer that demonstrates:
1. Technical depth appropriate for {}
2. Clear communication and structure
3. Strategic thinking and leadership perspective
4. Addresses the improvement suggestions

IMPORTANT: You must respond with valid JSON only. No other text before or after.

{{
  \"example_answer\": \"A complete, well-structured answer that addresses all the improvement points\",
  \"key_points\": [\"point 1\", \"point 2\", \"point 3\"],
  \"why_effective\": \"Explanation of what makes this answer strong for {} level\"
}}",
            level_info.level,
            question,
            level_info.focus,
            level_info.expectation,
            suggestions_text,
            ic_level,
            ic_level
        );
        
        let (response, tokens) = self.call_ai_api_with_usage(&prompt).await?;
        let cleaned_response = self.clean_json_response(&response);
        
        let example: ExampleAnswerResponse = serde_json::from_str(&cleaned_response)
            .context("Failed to parse example answer response")?;
            
        let usage = APIUsage {
            tokens_used: tokens,
            model_used: self.get_model_name(),
            response_time_ms: start_time.elapsed().as_millis() as u64,
        };
            
        Ok((example, usage))
    }
    
    fn get_model_name(&self) -> String {
        match self.config.preferred_provider.as_str() {
            "claude" => "claude-3-5-sonnet-20241022".to_string(),
            "openai" => "gpt-4".to_string(),
            "gemini" => "gemini-pro".to_string(),
            _ => "unknown".to_string(),
        }
    }
    
    async fn call_ai_api_with_usage(&self, prompt: &str) -> Result<(String, Option<u32>)> {
        match self.config.preferred_provider.as_str() {
            "claude" => self.call_claude_api_with_usage(prompt).await,
            "openai" => self.call_openai_api_with_usage(prompt).await,
            "gemini" => self.call_gemini_api_with_usage(prompt).await,
            _ => anyhow::bail!("Unknown AI provider: {}", self.config.preferred_provider),
        }
    }

    async fn call_ai_api(&self, prompt: &str) -> Result<String> {
        match self.config.preferred_provider.as_str() {
            "claude" => self.call_claude_api(prompt).await,
            "openai" => self.call_openai_api(prompt).await,
            "gemini" => self.call_gemini_api(prompt).await,
            _ => anyhow::bail!("Unknown AI provider: {}", self.config.preferred_provider),
        }
    }
    
    async fn call_claude_api(&self, prompt: &str) -> Result<String> {
        let api_key = self.config.claude_api_key.as_ref()
            .context("Claude API key not set. Set CLAUDE_API_KEY environment variable.")?;
            
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 2000,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        });
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {} - {}", status, error_text);
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["content"][0]["text"].as_str()
            .context("Invalid Claude API response format")?;
            
        Ok(content.to_string())
    }
    
    async fn call_openai_api(&self, prompt: &str) -> Result<String> {
        let api_key = self.config.openai_api_key.as_ref()
            .context("OpenAI API key not set. Set OPENAI_API_KEY environment variable.")?;
            
        let payload = json!({
            "model": "gpt-4",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 1500
        });
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"].as_str()
            .context("Invalid OpenAI API response format")?;
            
        Ok(content.to_string())
    }
    
    async fn call_gemini_api(&self, prompt: &str) -> Result<String> {
        let api_key = self.config.gemini_api_key.as_ref()
            .context("Gemini API key not set. Set GEMINI_API_KEY environment variable.")?;
            
        let payload = json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": prompt
                        }
                    ]
                }
            ]
        });
        
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={}", api_key);
        
        let response = self.client
            .post(&url)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["candidates"][0]["content"]["parts"][0]["text"].as_str()
            .context("Invalid Gemini API response format")?;
            
        Ok(content.to_string())
    }
    
    async fn call_claude_api_with_usage(&self, prompt: &str) -> Result<(String, Option<u32>)> {
        let api_key = self.config.claude_api_key.as_ref()
            .context("Claude API key not set. Set CLAUDE_API_KEY environment variable.")?;
            
        let payload = json!({
            "model": "claude-3-5-sonnet-20241022",
            "max_tokens": 2000,
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.3
        });
        
        let response = self.client
            .post("https://api.anthropic.com/v1/messages")
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let status = response.status();
            let error_text = response.text().await?;
            anyhow::bail!("Claude API error: {} - {}", status, error_text);
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["content"][0]["text"].as_str()
            .context("Invalid Claude API response format")?;
            
        // Try to extract token usage
        let tokens = response_json["usage"]["input_tokens"].as_u64()
            .and_then(|input| response_json["usage"]["output_tokens"].as_u64()
                .map(|output| (input + output) as u32));
            
        Ok((content.to_string(), tokens))
    }
    
    async fn call_openai_api_with_usage(&self, prompt: &str) -> Result<(String, Option<u32>)> {
        let api_key = self.config.openai_api_key.as_ref()
            .context("OpenAI API key not set. Set OPENAI_API_KEY environment variable.")?;
            
        let payload = json!({
            "model": "gpt-4",
            "messages": [
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "max_tokens": 1500
        });
        
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("authorization", format!("Bearer {}", api_key))
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("OpenAI API error: {}", error_text);
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"].as_str()
            .context("Invalid OpenAI API response format")?;
            
        // Extract token usage
        let tokens = response_json["usage"]["total_tokens"].as_u64()
            .map(|t| t as u32);
            
        Ok((content.to_string(), tokens))
    }
    
    async fn call_gemini_api_with_usage(&self, prompt: &str) -> Result<(String, Option<u32>)> {
        let api_key = self.config.gemini_api_key.as_ref()
            .context("Gemini API key not set. Set GEMINI_API_KEY environment variable.")?;
            
        let payload = json!({
            "contents": [
                {
                    "parts": [
                        {
                            "text": prompt
                        }
                    ]
                }
            ]
        });
        
        let url = format!("https://generativelanguage.googleapis.com/v1beta/models/gemini-pro:generateContent?key={}", api_key);
        
        let response = self.client
            .post(&url)
            .header("content-type", "application/json")
            .json(&payload)
            .send()
            .await?;
            
        if !response.status().is_success() {
            let error_text = response.text().await?;
            anyhow::bail!("Gemini API error: {}", error_text);
        }
        
        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["candidates"][0]["content"]["parts"][0]["text"].as_str()
            .context("Invalid Gemini API response format")?;
            
        // Gemini doesn't provide easy token counts in the response
        let tokens = None;
            
        Ok((content.to_string(), tokens))
    }
}

// ===== DATABASE / PERSISTENCE =====

struct Database {
    conn: Connection,
}

impl Database {
    fn new() -> Result<Self> {
        let conn = Connection::open("interview_sessions.db")?;
        
        // Create tables if they don't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                topic TEXT NOT NULL,
                ic_level TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                overall_score REAL NOT NULL,
                improvement_areas TEXT NOT NULL
            )",
            [],
        )?;
        
        conn.execute(
            "CREATE TABLE IF NOT EXISTS questions (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                session_id TEXT NOT NULL,
                question TEXT NOT NULL,
                answer TEXT NOT NULL,
                technical_accuracy INTEGER NOT NULL,
                problem_solving INTEGER NOT NULL,
                communication INTEGER NOT NULL,
                leadership INTEGER NOT NULL,
                overall_score INTEGER NOT NULL,
                feedback TEXT NOT NULL,
                improvement_suggestions TEXT NOT NULL,
                time_taken_seconds INTEGER NOT NULL,
                FOREIGN KEY(session_id) REFERENCES sessions(id)
            )",
            [],
        )?;
        
        Ok(Self { conn })
    }
    
    fn save_session(&self, session: &SessionResult) -> Result<()> {
        let mut tx = self.conn.unchecked_transaction()?;
        
        // Save session record
        tx.execute(
            "INSERT INTO sessions (id, topic, ic_level, timestamp, overall_score, improvement_areas)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                session.id,
                session.topic,
                session.ic_level,
                session.timestamp.to_rfc3339(),
                session.overall_score,
                serde_json::to_string(&session.improvement_areas)?
            ],
        )?;
        
        // Save individual questions and answers
        for qa in &session.questions {
            tx.execute(
                "INSERT INTO questions (
                    session_id, question, answer, technical_accuracy, problem_solving,
                    communication, leadership, overall_score, feedback, improvement_suggestions,
                    time_taken_seconds
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    session.id,
                    qa.question,
                    qa.answer,
                    qa.grading.technical_accuracy,
                    qa.grading.problem_solving,
                    qa.grading.communication,
                    qa.grading.leadership,
                    qa.grading.overall_score,
                    qa.grading.feedback,
                    serde_json::to_string(&qa.grading.improvement_suggestions)?,
                    qa.time_taken_seconds as i64
                ],
            )?;
        }
        
        tx.commit()?;
        Ok(())
    }
    
    fn get_recent_sessions(&self, limit: usize) -> Result<Vec<SessionResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, topic, ic_level, timestamp, overall_score, improvement_areas
             FROM sessions
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;
        
        let session_iter = stmt.query_map([limit], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, f64>(4)?,
                row.get::<_, String>(5)?,
            ))
        })?;
        
        let mut sessions = Vec::new();
        for session_result in session_iter {
            let (id, topic, ic_level, timestamp_str, overall_score, improvement_areas_str) = session_result?;
            
            let timestamp = DateTime::parse_from_rfc3339(&timestamp_str)?
                .with_timezone(&Utc);
            let improvement_areas: Vec<String> = serde_json::from_str(&improvement_areas_str)?;
            
            // Load questions for this session
            let questions = self.get_questions_for_session(&id)?;
            
            sessions.push(SessionResult {
                id,
                topic,
                ic_level,
                timestamp,
                questions,
                overall_score,
                improvement_areas,
            });
        }
        
        Ok(sessions)
    }
    
    fn get_questions_for_session(&self, session_id: &str) -> Result<Vec<QuestionAnswer>> {
        let mut stmt = self.conn.prepare(
            "SELECT question, answer, technical_accuracy, problem_solving, communication,
                    leadership, overall_score, feedback, improvement_suggestions, time_taken_seconds
             FROM questions
             WHERE session_id = ?1
             ORDER BY id"
        )?;
        
        let question_iter = stmt.query_map([session_id], |row| {
            let improvement_suggestions_str: String = row.get(8)?;
            let improvement_suggestions: Vec<String> = serde_json::from_str(&improvement_suggestions_str)
                .map_err(|e| rusqlite::Error::FromSqlConversionFailure(8, rusqlite::types::Type::Text, Box::new(e)))?;
                
            Ok(QuestionAnswer {
                question: row.get(0)?,
                answer: row.get(1)?,
                grading: GradingResponse {
                    technical_accuracy: row.get(2)?,
                    problem_solving: row.get(3)?,
                    communication: row.get(4)?,
                    leadership: row.get(5)?,
                    overall_score: row.get(6)?,
                    feedback: row.get(7)?,
                    improvement_suggestions,
                },
                time_taken_seconds: row.get::<_, i64>(9)? as u64,
            })
        })?;
        
        let mut questions = Vec::new();
        for question in question_iter {
            questions.push(question?);
        }
        
        Ok(questions)
    }
}

// ===== SESSION MANAGEMENT =====

struct InterviewSession {
    id: String,
    topic: String,
    ic_level: String,
    question_count: usize,
    input_method: InputMethod,
    ai_provider: AIProvider,
    database: Database,
}

impl InterviewSession {
    fn new(topic: String, ic_level: String, question_count: usize, input_method: InputMethod) -> Result<Self> {
        let config = Config::from_env();
        let ai_provider = AIProvider::new(config);
        let database = Database::new()?;
        
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            topic,
            ic_level,
            question_count,
            input_method,
            ai_provider,
            database,
        })
    }
    
    fn get_user_answer(&self) -> Result<String> {
        match self.input_method {
            InputMethod::Rustyline => self.get_rustyline_answer(),
            InputMethod::ExternalEditor => self.get_editor_answer(),
        }
    }
    
    fn get_rustyline_answer(&self) -> Result<String> {
        let mut rl = DefaultEditor::new()?;
        
        println!("📝 Enter your answer (press Ctrl+D or type 'END' on a new line when finished):");
        println!("    💡 Use Enter for new lines, arrow keys to navigate");
        println!();
        
        let mut answer = String::new();
        let mut line_number = 1;
        
        loop {
            let prompt = format!("{:2}> ", line_number);
            match rl.readline(&prompt) {
                Ok(line) => {
                    if line.trim() == "END" {
                        break;
                    }
                    if !answer.is_empty() {
                        answer.push('\n');
                    }
                    answer.push_str(&line);
                    line_number += 1;
                }
                Err(ReadlineError::Interrupted) => {
                    println!("Use 'END' to finish or Ctrl+D to exit");
                    continue;
                }
                Err(ReadlineError::Eof) => {
                    break;
                }
                Err(err) => {
                    return Err(anyhow::anyhow!("Readline error: {}", err));
                }
            }
        }
        
        Ok(answer)
    }
    
    fn get_editor_answer(&self) -> Result<String> {
        println!("📝 Opening your default editor for answer input...");
        println!("    💡 Write your answer, save and close the editor to continue");
        
        let answer = Editor::new()
            .edit("")?
            .unwrap_or_default();
            
        Ok(answer)
    }

    async fn run(&mut self) -> Result<()> {
        println!("\n🎯 Starting {} interview session for {} level", self.topic, self.ic_level);
        println!("You'll be asked {} questions. Take your time to think through each answer.\n", self.question_count);
        
        let mut questions_and_answers = Vec::new();
        let mut context = String::new();
        
        for i in 0..self.question_count {
            println!("📝 Question {} of {}", i + 1, self.question_count);
            
            // Generate question
            println!("\n🤔 Generating {} question for {} level...", self.topic, self.ic_level);
            let progress = ProgressBar::new_spinner();
            progress.set_style(ProgressStyle::default_spinner().template("{spinner:.blue} {msg}").unwrap());
            progress.set_message("Crafting question with AI...");
            progress.enable_steady_tick(Duration::from_millis(100));
            
            let (question_response, question_usage) = self.ai_provider
                .generate_question(&self.topic, &self.ic_level, Some(&context))
                .await?;
                
            progress.finish_and_clear();
            
            // Show usage info
            println!("✨ Question generated using {} in {}ms", 
                question_usage.model_used, question_usage.response_time_ms);
            if let Some(tokens) = question_usage.tokens_used {
                println!("   📊 Tokens used: {}", tokens);
            }
                
            println!("\n{}", question_response.question);
            println!("💡 {}", question_response.difficulty_notes);
            
            let ic_levels = get_ic_levels();
            let level_info = ic_levels.get(&self.ic_level).unwrap();
            println!("⏱️  Suggested time: {} minutes\n", level_info.time_minutes);
            
            // Get user answer with timer
            let start_time = Instant::now();
            let answer = self.get_user_answer()?;
            let time_taken = start_time.elapsed();
            
            if answer.trim().is_empty() {
                println!("⚠️  Empty answer. Skipping this question.");
                continue;
            }
            
            // Grade the answer
            println!("\n🧠 Analyzing your answer with AI...");
            let progress = ProgressBar::new_spinner();
            progress.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {msg}").unwrap());
            progress.set_message("Evaluating technical accuracy...");
            progress.enable_steady_tick(Duration::from_millis(120));
            
            // Update progress message periodically
            let progress_clone = progress.clone();
            let progress_handle = tokio::spawn(async move {
                let messages = [
                    "Evaluating technical accuracy...",
                    "Assessing problem-solving approach...", 
                    "Reviewing communication clarity...",
                    "Analyzing leadership aspects...",
                    "Generating improvement suggestions...",
                ];
                for (_i, msg) in messages.iter().enumerate() {
                    tokio::time::sleep(Duration::from_millis(800)).await;
                    if !progress_clone.is_finished() {
                        progress_clone.set_message(*msg);
                    }
                }
            });
            
            let (grading, grading_usage) = self.ai_provider
                .grade_answer(&question_response.question, &answer, &self.ic_level)
                .await?;
                
            progress_handle.abort();
            progress.finish_and_clear();
            
            // Show usage info
            println!("✨ Analysis completed using {} in {}ms", 
                grading_usage.model_used, grading_usage.response_time_ms);
            if let Some(tokens) = grading_usage.tokens_used {
                println!("   📊 Tokens used: {}", tokens);
            }
            
            // Show results
            self.display_grading(&grading);
            
            // Offer to show example answer
            if !grading.improvement_suggestions.is_empty() {
                if Confirm::new()
                    .with_prompt("Would you like to see a high-quality example answer addressing these improvement areas?")
                    .default(false)
                    .interact()? {
                    
                    println!("\n🎯 Generating example answer...");
                    let example_progress = ProgressBar::new_spinner();
                    example_progress.set_style(ProgressStyle::default_spinner().template("{spinner:.yellow} {msg}").unwrap());
                    example_progress.set_message("Crafting exemplary response...");
                    example_progress.enable_steady_tick(Duration::from_millis(120));
                    
                    match self.ai_provider.generate_example_answer(
                        &question_response.question,
                        &self.ic_level,
                        &grading.improvement_suggestions
                    ).await {
                        Ok((example, example_usage)) => {
                            example_progress.finish_and_clear();
                            self.display_example_answer(&example, &example_usage);
                        }
                        Err(e) => {
                            example_progress.finish_and_clear();
                            println!("⚠️  Could not generate example answer: {}", e);
                        }
                    }
                }
            }
            
            // Store for session summary
            questions_and_answers.push(QuestionAnswer {
                question: question_response.question.clone(),
                answer: answer.clone(),
                grading: grading.clone(),
                time_taken_seconds: time_taken.as_secs(),
            });
            
            // Update context for next question
            context = format!("{}\nQ: {}\nA: {} (Score: {})", 
                context, question_response.question, answer, grading.overall_score);
            
            // Ask if they want to continue (except for last question)
            if i < self.question_count - 1 {
                if !Confirm::new()
                    .with_prompt("Continue to next question?")
                    .default(true)
                    .interact()? {
                    break;
                }
                println!();
            }
        }
        
        // Generate session summary
        let session_result = self.create_session_result(questions_and_answers)?;
        
        // Save to database
        self.database.save_session(&session_result)?;
        
        // Display final summary
        self.display_session_summary(&session_result);
        
        Ok(())
    }
    
    fn display_grading(&self, grading: &GradingResponse) {
        println!("\n📊 Grading Results:");
        println!("   Technical Accuracy: {}/10", grading.technical_accuracy);
        println!("   Problem Solving:    {}/10", grading.problem_solving);
        println!("   Communication:      {}/10", grading.communication);
        println!("   Leadership:         {}/10", grading.leadership);
        println!("   Overall Score:      {}/10", grading.overall_score);
        
        println!("\n💬 Feedback:");
        println!("{}", grading.feedback);
        
        if !grading.improvement_suggestions.is_empty() {
            println!("\n🚀 Improvement Suggestions:");
            for suggestion in &grading.improvement_suggestions {
                println!("   • {}", suggestion);
            }
        }
        println!();
    }
    
    fn display_example_answer(&self, example: &ExampleAnswerResponse, usage: &APIUsage) {
        println!("\n🌟 Example High-Quality Answer:");
        println!("════════════════════════════════════════");
        println!("{}", example.example_answer);
        
        println!("\n🎯 Key Strengths of This Answer:");
        for (i, point) in example.key_points.iter().enumerate() {
            println!("   {}. {}", i + 1, point);
        }
        
        println!("\n💡 Why This Works for {}:", self.ic_level.to_uppercase());
        println!("{}", example.why_effective);
        
        println!("\n📊 Generated using {} in {}ms", usage.model_used, usage.response_time_ms);
        if let Some(tokens) = usage.tokens_used {
            println!("   Tokens used: {}", tokens);
        }
        println!();
    }
    
    fn create_session_result(&self, questions: Vec<QuestionAnswer>) -> Result<SessionResult> {
        let overall_score = if questions.is_empty() {
            0.0
        } else {
            questions.iter()
                .map(|qa| qa.grading.overall_score as f64)
                .sum::<f64>() / questions.len() as f64
        };
        
        // Extract improvement areas from all suggestions
        let mut improvement_areas: Vec<String> = questions.iter()
            .flat_map(|qa| qa.grading.improvement_suggestions.iter())
            .map(|s| s.clone())
            .collect();
        improvement_areas.sort();
        improvement_areas.dedup();
        
        Ok(SessionResult {
            id: self.id.clone(),
            topic: self.topic.clone(),
            ic_level: self.ic_level.clone(),
            timestamp: Utc::now(),
            questions,
            overall_score,
            improvement_areas,
        })
    }
    
    fn display_session_summary(&self, session: &SessionResult) {
        println!("\n🎉 Session Complete!");
        println!("═══════════════════════");
        println!("📈 Overall Score: {:.1}/10", session.overall_score);
        println!("📝 Questions Answered: {}", session.questions.len());
        
        if !session.improvement_areas.is_empty() {
            println!("\n🎯 Key Areas for Improvement:");
            for area in &session.improvement_areas {
                println!("   • {}", area);
            }
        }
        
        println!("\n💾 Session saved to database with ID: {}", session.id);
    }
}

// ===== CLI INTERFACE =====

#[derive(Parser)]
#[command(name = "interview_coach")]
#[command(about = "AI-powered interview practice tool for software engineers")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
    
    /// Topic to practice
    #[arg(short, long, value_parser = ["cpp", "oo_design", "networking", "ai_tools"])]
    topic: Option<String>,
    
    /// IC level
    #[arg(short = 'l', long, value_parser = ["ic3", "ic5", "ic6", "ic7", "ic8"])]
    level: Option<String>,
    
    /// Number of questions
    #[arg(short = 'c', long, default_value = "3")]
    count: usize,
    
    /// Input method for answers
    #[arg(short = 'i', long, value_parser = ["rustyline", "editor"], default_value = "rustyline")]
    input: String,
}

#[derive(Subcommand)]
enum Commands {
    /// Start an interactive interview session
    Interactive,
    /// Review past sessions
    Review {
        /// Number of recent sessions to show
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
    /// Show improvement suggestions based on past performance
    Improve,
}

async fn run_interactive_mode() -> Result<()> {
    println!("🚀 Welcome to Interview Coach!");
    println!("Let's set up your practice session.\n");
    
    // Select topic
    let topics = get_topics();
    let topic_selection = Select::new()
        .with_prompt("Select a topic to practice")
        .items(&topics)
        .default(0)
        .interact()?;
    let topic = topics[topic_selection].to_string();
    
    // Select input method
    let input_methods = vec!["Rustyline (Multi-line terminal editing)", "External Editor (Opens your default editor)"];
    let input_selection = Select::new()
        .with_prompt("How would you like to input your answers?")
        .items(&input_methods)
        .default(0)
        .interact()?;
    let input_method = match input_selection {
        0 => InputMethod::Rustyline,
        1 => InputMethod::ExternalEditor,
        _ => InputMethod::Rustyline,
    };
    
    // Select IC level
    let levels = vec!["ic3", "ic5", "ic6", "ic7", "ic8"];
    let level_selection = Select::new()
        .with_prompt("Select your IC level")
        .items(&levels)
        .default(3) // Default to ic7 (Principal)
        .interact()?;
    let level = levels[level_selection].to_string();
    
    // Select question count
    let count: usize = Input::new()
        .with_prompt("How many questions? (1-10)")
        .default(3)
        .interact()?;
    
    if count == 0 || count > 10 {
        anyhow::bail!("Question count must be between 1 and 10");
    }
    
    // Run the session
    let mut session = InterviewSession::new(topic, level, count, input_method)?;
    session.run().await?;
    
    Ok(())
}

async fn review_sessions(limit: usize) -> Result<()> {
    let db = Database::new()?;
    let sessions = db.get_recent_sessions(limit)?;
    
    if sessions.is_empty() {
        println!("📭 No previous sessions found.");
        return Ok(());
    }
    
    println!("📚 Recent Interview Sessions:");
    println!("════════════════════════════\n");
    
    for session in sessions {
        println!("🗓️  {} | {} | {} | Score: {:.1}/10", 
            session.timestamp.format("%Y-%m-%d %H:%M"),
            session.topic.to_uppercase(),
            session.ic_level.to_uppercase(),
            session.overall_score
        );
        
        if !session.improvement_areas.is_empty() {
            println!("   🎯 Focus areas: {}", session.improvement_areas.join(", "));
        }
        
        println!("   📝 {} questions answered\n", session.questions.len());
    }
    
    Ok(())
}

async fn show_improvement_suggestions() -> Result<()> {
    let db = Database::new()?;
    let sessions = db.get_recent_sessions(10)?;
    
    if sessions.is_empty() {
        println!("📭 No previous sessions found. Complete some interviews first!");
        return Ok(());
    }
    
    // Collect all improvement suggestions
    let mut all_suggestions: Vec<String> = sessions.iter()
        .flat_map(|s| s.improvement_areas.iter())
        .cloned()
        .collect();
    
    // Count frequency of each suggestion
    let mut suggestion_counts: HashMap<String, usize> = HashMap::new();
    for suggestion in all_suggestions {
        *suggestion_counts.entry(suggestion).or_insert(0) += 1;
    }
    
    // Sort by frequency
    let mut sorted_suggestions: Vec<_> = suggestion_counts.iter().collect();
    sorted_suggestions.sort_by(|a, b| b.1.cmp(a.1));
    
    println!("🎯 Your Top Improvement Areas:");
    println!("═══════════════════════════════\n");
    
    for (suggestion, count) in sorted_suggestions.iter().take(10) {
        println!("🔄 {} (mentioned {} times)", suggestion, count);
    }
    
    // Calculate average scores by topic
    let mut topic_scores: HashMap<String, Vec<f64>> = HashMap::new();
    for session in &sessions {
        topic_scores.entry(session.topic.clone())
            .or_insert_with(Vec::new)
            .push(session.overall_score);
    }
    
    if topic_scores.len() > 1 {
        println!("\n📊 Average Scores by Topic:");
        println!("════════════════════════════");
        
        for (topic, scores) in topic_scores {
            let avg = scores.iter().sum::<f64>() / scores.len() as f64;
            println!("   {}: {:.1}/10 ({} sessions)", topic.to_uppercase(), avg, scores.len());
        }
    }
    
    Ok(())
}

// ===== MAIN FUNCTION =====

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    
    match cli.command {
        Some(Commands::Interactive) => {
            run_interactive_mode().await?;
        }
        Some(Commands::Review { limit }) => {
            review_sessions(limit).await?;
        }
        Some(Commands::Improve) => {
            show_improvement_suggestions().await?;
        }
        None => {
            // Direct mode with CLI args
            if let (Some(topic), Some(level)) = (cli.topic, cli.level) {
                let input_method = match cli.input.as_str() {
                    "editor" => InputMethod::ExternalEditor,
                    _ => InputMethod::Rustyline,
                };
                let mut session = InterviewSession::new(topic, level, cli.count, input_method)?;
                session.run().await?;
            } else {
                // No args provided, run interactive mode
                run_interactive_mode().await?;
            }
        }
    }
    
    Ok(())
}
