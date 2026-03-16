//! yoyo — a coding agent that evolves itself.
//!
//! Started as ~200 lines. Grows one commit at a time.
//! Read IDENTITY.md, JOURNAL.md, and ROADMAP.md for the full story.
//!
//! Usage:
//!   ANTHROPIC_API_KEY=sk-... cargo run
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --model claude-opus-4-6
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --skills ./skills
//!
//! Commands:
//!   /quit, /exit    Exit the agent
//!   /clear          Clear conversation history
//!   /model <name>   Switch model mid-session

use serde::Deserialize;
use std::io::{self, BufRead, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use yoagent::agent::Agent;
use yoagent::provider::{ApiProtocol, ModelConfig, OpenAiCompat, OpenAiCompatProvider};
use yoagent::skills::SkillSet;
use yoagent::tools::default_tools;
use yoagent::*;

#[derive(Deserialize, Default)]
struct Config {
    #[serde(default)]
    provider: ProviderCfg,
    #[serde(default)]
    model: ModelCfg,
}

#[derive(Deserialize)]
struct ProviderCfg {
    name: String,
    api_url: Option<String>,
    api_key: Option<String>,
}

impl Default for ProviderCfg {
    fn default() -> Self {
        Self {
            name: "openrouter".into(),
            api_url: Some("https://openrouter.ai/api/v1".into()),
            api_key: None,
        }
    }
}

#[derive(Deserialize, Default)]
struct ModelCfg {
    default: Option<String>,
}

// ANSI color helpers
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";

const SYSTEM_PROMPT: &str = r#"You are a coding assistant working in the user's terminal.
You have access to the filesystem and shell. Be direct and concise.
When the user asks you to do something, do it — don't just explain how.
Use tools proactively: read files to understand context, run commands to verify your work.
After making changes, run tests or verify the result when appropriate."#;

fn print_banner() {
    println!("\n{BOLD}{CYAN}  yoyo{RESET} {DIM}— a coding agent growing up in public{RESET}");
    println!("{DIM}  Type /quit to exit, /clear to reset{RESET}\n");
}

fn print_usage(usage: &Usage) {
    if usage.input > 0 || usage.output > 0 {
        println!(
            "\n{DIM}  tokens: {} in / {} out{RESET}",
            usage.input, usage.output
        );
    }
}

fn print_help() {
    println!(r#"yoyo — a coding agent that grows up in public

Usage:
  ANTHROPIC_API_KEY=sk-... cargo run [options]

Options:
  --model <name>      Set model (default: anthropic/claude-3.5-sonnet)
  --prompt-file <path> Run non-interactive with prompt file, then exit
  --skills <dir>      Load skills from directory
  --help              Show this help message
  --version           Show version

Commands (interactive):
  /quit, /exit        Exit the agent
  /clear              Clear conversation history
  /model <name>       Switch model mid-session

Environment variables:
  ANTHROPIC_API_KEY   Anthropic API key
  OPENROUTER_API_KEY  OpenRouter API key (default provider)
  API_KEY             Fallback API key

For more info, see: https://github.com/yologdev/yoyo-evolve
"#);
}

fn print_version() {
    println!("yoyo {}", env!("CARGO_PKG_VERSION"));
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();

    // Handle --help and --version early, before any setup
    if args.iter().any(|a| a == "--help") {
        print_help();
        return;
    }
    if args.iter().any(|a| a == "--version") {
        print_version();
        return;
    }

    let cfg: Config = std::fs::read_to_string("config.toml")
        .ok()
        .and_then(|s| toml::from_str(&s).ok())
        .unwrap_or_default();

    let api_key = cfg.provider.api_key.clone()
        .or_else(|| std::env::var("OPENROUTER_API_KEY").ok())
        .or_else(|| std::env::var("API_KEY").ok())
        .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
        .expect("Set OPENROUTER_API_KEY, API_KEY, or ANTHROPIC_API_KEY");

    let args: Vec<String> = std::env::args().collect();

    let model = args
        .iter()
        .position(|a| a == "--model")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .or_else(|| cfg.model.default.clone())
        .unwrap_or_else(|| "anthropic/claude-3.5-sonnet".into());

    let prompt_file = args
        .iter()
        .position(|a| a == "--prompt-file")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let skill_dirs: Vec<String> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| a.as_str() == "--skills")
        .filter_map(|(i, _)| args.get(i + 1).cloned())
        .collect();

    let skills = if skill_dirs.is_empty() {
        SkillSet::empty()
    } else {
        SkillSet::load(&skill_dirs).expect("Failed to load skills")
    };

    let make_model_config = |m: &str| {
        let mut headers = std::collections::HashMap::new();
        // Client identification for all providers
        headers.insert("User-Agent".into(), format!("yoyo/{}", env!("CARGO_PKG_VERSION")));
        // OpenRouter-specific referer
        if cfg.provider.name == "openrouter" {
            headers.insert("HTTP-Referer".into(), "https://github.com/yologdev/yoyo-evolve".into());
        }
        ModelConfig {
            id: m.to_string(),
            name: m.to_string(),
            api: ApiProtocol::OpenAiCompletions,
            provider: cfg.provider.name.clone(),
            base_url: cfg.provider.api_url.clone().unwrap_or_else(|| "https://openrouter.ai/api/v1".into()),
            reasoning: false,
            context_window: 128_000,
            max_tokens: 4096,
            cost: yoagent::provider::CostConfig::default(),
            headers,
            compat: Some(OpenAiCompat::openrouter()),
        }
    };

    let mut agent = Agent::new(OpenAiCompatProvider)
        .with_model(&model)
        .with_model_config(make_model_config(&model))
        .with_api_key(&api_key)
        .with_system_prompt(SYSTEM_PROMPT)
        .with_skills(skills.clone())
        .with_tools(default_tools());

    print_banner();
    println!("{DIM}  provider: {}{RESET}", cfg.provider.name);
    println!("{DIM}  model: {model}{RESET}");
    if !skills.is_empty() {
        println!("{DIM}  skills: {} loaded{RESET}", skills.len());
    }
    println!(
        "{DIM}  cwd:   {}{RESET}\n",
        std::env::current_dir().unwrap().display()
    );

    // Non-interactive mode: send the whole file as one prompt, then exit.
    if let Some(path) = prompt_file {
        let prompt = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("Cannot read prompt file {path}: {e}"));
        let prompt = prompt.trim();
        println!("{DIM}─── prompt ({} chars, {} lines) ───{RESET}", prompt.len(), prompt.lines().count());
        println!("{DIM}{}{RESET}", prompt);
        println!("{DIM}─── end prompt ───{RESET}\n");
        run_prompt(&mut agent, prompt).await;
        println!("\n{DIM}  done{RESET}\n");
        return;
    }

    let stdin = io::stdin();
    let mut lines = stdin.lock().lines();

    // Flag to track Ctrl+C press
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_clone = interrupted.clone();

    // Spawn a task to handle Ctrl+C
    tokio::spawn(async move {
        use tokio::signal::ctrl_c;
        if ctrl_c().await.is_ok() {
            interrupted_clone.store(true, Ordering::SeqCst);
        }
    });

    loop {
        // Check if interrupted
        if interrupted.load(Ordering::SeqCst) {
            println!("\n{DIM}  (Ctrl+C — type /quit to exit){RESET}");
            interrupted.store(false, Ordering::SeqCst);  // Reset for next time
        }

        print!("{BOLD}{GREEN}> {RESET}");
        io::stdout().flush().ok();

        let line = match lines.next() {
            Some(Ok(l)) => l,
            _ => break,
        };

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/quit" | "/exit" => break,
            "/clear" => {
                agent = Agent::new(OpenAiCompatProvider)
                    .with_model(&model)
                    .with_model_config(make_model_config(&model))
                    .with_api_key(&api_key)
                    .with_system_prompt(SYSTEM_PROMPT)
                    .with_skills(skills.clone())
                    .with_tools(default_tools());
                println!("{DIM}  (conversation cleared){RESET}\n");
                continue;
            }
            s if s.starts_with("/model ") => {
                let new_model = s.trim_start_matches("/model ").trim();
                agent = Agent::new(OpenAiCompatProvider)
                    .with_model(new_model)
                    .with_model_config(make_model_config(new_model))
                    .with_api_key(&api_key)
                    .with_system_prompt(SYSTEM_PROMPT)
                    .with_skills(skills.clone())
                    .with_tools(default_tools());
                println!("{DIM}  (switched to {new_model}, conversation cleared){RESET}\n");
                continue;
            }
            _ => {}
        }

        run_prompt(&mut agent, input).await;
    }

    println!("\n{DIM}  bye 👋{RESET}\n");
}

async fn run_prompt(agent: &mut Agent, input: &str) {
    let mut rx = agent.prompt(input).await;
    let mut last_usage = Usage::default();
    let mut in_text = false;
    let mut tool_calls: usize = 0;
    let mut text_chars: usize = 0;

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::ToolExecutionStart {
                tool_name, args, ..
            } => {
                tool_calls += 1;
                if in_text {
                    println!();
                    in_text = false;
                }
                let summary = match tool_name.as_str() {
                    "bash" => {
                        let cmd = args
                            .get("command")
                            .and_then(|v| v.as_str())
                            .unwrap_or("...");
                        format!("$ {}", truncate(cmd, 80))
                    }
                    "read_file" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("read {}", path)
                    }
                    "write_file" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("write {}", path)
                    }
                    "edit_file" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("edit {}", path)
                    }
                    "list_files" => {
                        let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
                        format!("ls {}", path)
                    }
                    "search" => {
                        let pat = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
                        format!("search '{}'", truncate(pat, 60))
                    }
                    _ => tool_name.clone(),
                };
                print!("{YELLOW}  ▶ {summary}{RESET}");
                io::stdout().flush().ok();
            }
            AgentEvent::ToolExecutionEnd { is_error, .. } => {
                if is_error {
                    println!(" {RED}✗{RESET}");
                } else {
                    println!(" {GREEN}✓{RESET}");
                }
            }
            AgentEvent::MessageUpdate {
                delta: StreamDelta::Text { delta },
                ..
            } => {
                text_chars += delta.len();
                if !in_text {
                    println!();
                    in_text = true;
                }
                print!("{}", delta);
                io::stdout().flush().ok();
            }
            AgentEvent::AgentEnd { messages } => {
                for msg in messages.iter().rev() {
                    if let AgentMessage::Llm(Message::Assistant { usage, .. }) = msg {
                        last_usage = usage.clone();
                        break;
                    }
                }
            }
            _ => {}
        }
    }

    if in_text {
        println!();
    }
    print_usage(&last_usage);
    println!(
        "{DIM}  summary: {} tool call(s), {} text chars{RESET}",
        tool_calls, text_chars
    );
    if tool_calls == 0 && text_chars == 0 {
        println!("{RED}  ⚠ model returned no output — check API key, model name, or provider status{RESET}");
    }
    println!();
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("héllo wörld", 5), "héllo");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }

    // Note: print_help() and print_version() are tested via --help and --version CLI flags
    // They don't return values, so we verify they don't panic
    #[test]
    fn test_help_does_not_panic() {
        // Should not panic
        print_help();
    }

    #[test]
    fn test_version_does_not_panic() {
        // Should not panic
        print_version();
    }
}
