# Omni-Clipboard & Instant AI Lens - Progress Log

## Status: Released (v0.1.0)
- **GitHub Repository**: https://github.com/pacjung64-cmyk/omni-clipboard
- **GitHub Release v0.1.0**: https://github.com/pacjung64-cmyk/omni-clipboard/releases/tag/v0.1.0
- **Direct Downloads**:
  - Standalone EXE: `omni-clipboard-v0.1.0-windows-x64.exe` (7.03 MB)
  - Portable ZIP: `omni-clipboard-v0.1.0-windows-x64.zip` (3.09 MB)

## Core Components
- [x] **Model & Sanitizer**: SHA-256 deduplication, content type classification (Text, Code, Url, Sensitive), zero-leak regex masking (API keys, JWT, Credit cards).
- [x] **Storage**: SQLite with FTS5 search index (`clips_fts`) + hand-written `FakeStorage` in-memory test fake.
- [x] **Win32 Hooks**: Background window message loop (`AddClipboardFormatListener`), Hotkeys `Ctrl+Shift+V` and `Alt+Space`, SendInput virtual paste.
- [x] **AI Lens**: Multi-provider client (Ollama, Gemini, Groq, OpenAI) with actions (Explain, TranslateThEn, FixCode, PolishTone, Custom).
- [x] **UI**: Standalone frameless Raycast acrylic dark mode via Microsoft Edge App Mode (`msedge.exe --app`).
- [x] **Tests**: 6/6 automated regression tests passing (`cargo test`).
- [x] **Distribution**: Release binary built, packaged, and published on GitHub Releases.
