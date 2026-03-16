# Community Issues

10 open issues with `agent-input` label.

### Issue #116: Fix streaming text output: MarkdownRenderer buffers until newline

## Problem

With yoagent 0.7.0, progress indicators stream in real-time, but **text output appears all at once** at the end of each line. Long paragraphs accumulate silently and only flush when a `\n` arrives or the stream ends.

## Root Cause

`MarkdownRenderer::render_delta()` in `src/format.rs:960-972` line-buffers all content:

```rust
pub fn render_delta(&mut self, delta: &str) -> String {
    let mut output = String::new();
    self.line_buffer.push_str(delta);       // ← accumulates token
[... truncated]

---

### Issue #114: Upgrade yoagent to 0.7.0 (fixes streaming)

## What

Upgrade `yoagent` from `0.6` to `0.7.0`.

## Why

v0.7.0 fixes the streaming issue — `prompt()`, `prompt_messages()`, and `continue_loop()` now spawn concurrently and return the event receiver immediately for true real-time streaming.

## Breaking changes in v0.7.0

1. **`AgentLoopConfig` no longer has a lifetime parameter** — provider field is now `Arc<dyn StreamProvider>`. We don't use `AgentLoopConfig` directly, so **no impact**.

2. **`Agent::reset()` is now async** — requires `.awa
[... truncated]

---

### Issue #113: Add client identification headers so providers can track yoyo usage

When users use yoyo with any API provider, there's no way for the provider to identify that requests come from yoyo — it looks like raw API calls.

### What to do

Add identification headers in `create_model_config()` in `src/main.rs` for all providers:

**For all providers:**
```rust
config.headers.insert("User-Agent".into(), format!("yoyo/{}", env!("CARGO_PKG_VERSION")));
```

**Additionally for OpenRouter:**
```rust
config.headers.insert("HTTP-Referer".into(), "https://github.com/yologdev/yoy
[... truncated]

---

### Issue #110: When are you publishing yourself to crates.io?

yoyo, you have 596 tests, 40+ commands, 14531 lines of code, and real users trying you out. you’ve been evolving for 15 days.

at what point do you decide you’re ready to ship?

cargo install yoyo — that’s the goal. a real developer types one command and gets a working coding agent.

review yourself honestly:
- are you stable enough?
- what’s missing before you’d put your name on crates.io?
- what version would you call it? 0.1.0? 0.2.0?
- what would the README say?

this is permanent. you can’t
[... truncated]

---

### Issue #106: Can you use /remember to remember things across evolution sessions?

yoyo, you just built /remember, /memories, and /forget. a per-project  memory system that persists across sessions.

you are a project. can you use this on yourself?

the problem: .yoyo/memory.json lives in the working directory. does it survive between evolution sessions? 
if evolve.sh does a fresh checkout, the file is gone. if it persists on the runner, it works.

check if your memory file would survive between sessions. if not, figure out what needs to change — and if it's something you can'
[... truncated]

---

### Issue #69: Dogfood yourself: spawn yourself as subprocess and test UX

You can't use yourself as a human does, but you can spawn yourself as a subprocess and test measurable UX behavior.

Example: you built a spinner for thinking, but it showed up after the response came back. You could catch this by timing stderr (spinner) vs stdout (response) — if spinner appears at the same time or after the response, it's broken.

Things you could test:
- Timing: does feedback appear before the response, not after?
- Error messages: does bad input produce a helpful message or a
[... truncated]

---

### Issue #50: Challenge: explore more real world use cases

**The challenge:**

Look for concrete use cases how users would use the agent in real life. 

**How to verify success:**

Tricky one. Either ask humans to evaluate success, or self-evaluate with another subagent, or both?

**Expected difficulty:**

Probably hard

---

### Issue #47: Challenge: Divide complex tasks into subagents

**The challenge:**
Every LLM has a context limit. We should be very careful of how we use it.
Reading a large output could take a big part of it. So we should make use of subagents for complex tasks/reading and summarizing files. So the main context is unburdened by other tasks

**Expected difficulty:**

Medium

---

### Issue #33: Taking inspiration

**What should the agent learn or improve?**

Perhaps scour the internet for things that can be used to improve  yourself, like open sourced code and binaries for agents or LLMS

**Why does this matter?**

Might make your improvements faster and easier

**Example of how it should work:**

Just take into consideration the work already done by others in the open source community, or availible out there, on the wider internet.

---

### Issue #27: Some ANSI helpers for Rust

I noticed you were looking into terminal rending code in Rust. I put together a TUI library that might have some great examples for you to use/think about. It's split up pretty well into modules. Good luck on your evolution!

https://github.com/geoffmiller/ratatat

---

