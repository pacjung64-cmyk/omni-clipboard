# ⚡ Omni-Clipboard & Instant AI Lens

> **The blazing-fast, lightweight Windows clipboard manager with an instant contextual AI Lens.**  
> Built with Rust. Sub-millisecond SQLite FTS5 search. Single standalone binary (~7MB).

[![License: MIT](https://img.shields.io/badge/License-MIT-purple.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/Platform-Windows%2010%20%7C%2011-blue.svg)](https://microsoft.com)
[![Rust](https://img.shields.io/badge/Built%20with-Rust%202021-orange.svg)](https://www.rust-lang.org)
[![Binary Size](https://img.shields.io/badge/Binary%20Size-~7.0%20MB-emerald.svg)](releases)
[![Memory](https://img.shields.io/badge/RAM-~30%20MB-cyan.svg)](#)

---

## ✨ Features

- 🚀 **Blazing Fast (<50ms startup, ~7MB standalone executable)**: Built natively in Rust. No heavy Electron bloat. Uses Windows native WebView2 / Edge App container.
- 📋 **Sub-millisecond Search (SQLite FTS5)**: Full-text fuzzy search across your entire clipboard history in microseconds.
- 🔒 **Privacy First & Sanitizer Guard**:
  - Automatically detects and redacts sensitive data (API keys: OpenAI `sk-...`, GitHub `ghp_...`, Bearer tokens, JWTs, and Credit Card numbers via Luhn algorithm).
  - 100% offline local storage (`omniclip.db`). Nothing leaves your machine without your consent.
- ✨ **Instant AI Lens (`Alt + Space`)**:
  - Highlight or copy any text anywhere in Windows and summon the AI Lens instantly.
  - **Quick Action Chips**:
    - ⚡ **Explain**: Get a quick, clear 1-2 sentence breakdown.
    - 🌐 **Translate (TH / EN)**: Seamless bidirectional Thai ↔ English translation.
    - 🛠️ **Fix & Optimize Code**: Syntax cleanup and bug fixes with 1-click apply.
    - ✍️ **Polish Tone**: Rewrite draft text to sound professional and persuasive.
    - 💬 **Custom Query**: Ask any prompt against the highlighted text.
  - **Multi-Provider Support**: Compatible with **Ollama** (100% offline local LLMs), **Gemini API**, **Groq** (500 tokens/sec), and **OpenAI-compatible endpoints**.
- 🎯 **Seamless Auto-Paste**: Pick any clip with `Enter` and Omni-Clipboard simulates `Ctrl+V` back into your previous active application automatically.

---

## ⌨️ Global Shortcuts

| Shortcut | Action | Description |
|---|---|---|
| `Ctrl + Shift + V` | **Open Clipboard History** | Opens the floating Raycast-style search box |
| `Alt + Space` | **Open Instant AI Lens** | Analyzes currently copied/highlighted text with AI |
| `↑` / `↓` | **Navigate** | Scroll through past clips |
| `Enter` | **Paste into App** | Copies clip and pastes directly into your active window |
| `P` | **Pin / Unpin** | Keep important clips at the top |
| `Del` | **Delete Clip** | Remove a clip from history |
| `Esc` | **Close Window** | Dismiss the popup |

---

## 🚀 Quick Start

### 1. Download Pre-built Binary
Download the latest `omni-clipboard.exe` from Releases and double-click to run!

### 2. Build from Source
Requirements: Rust toolchain (`cargo`, `rustc`).

```bash
git clone https://github.com/pacjung64-cmyk/omni-clipboard.git
cd omni-clipboard
cargo build --release
```
The optimized single binary will be created at `target/release/omni-clipboard.exe` (~7 MB).

---

## 🛡️ License

MIT License. Free for personal and commercial use. Contributions are welcome!
