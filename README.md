# Sunny's Whisper

Free, local speech-to-text for macOS — a customized fork of [nobs-whisper](https://github.com/team-attention/nobs-whisper).

No subscriptions, no API keys, no BS. Everything runs locally on your Mac.

## Download

**[Download Latest Release](https://github.com/SHINJUNGSUN/sunny-whisper/releases/latest)**

## Features (Original)

- **100% Local** — OpenAI Whisper models running entirely on your device
- **Metal GPU Acceleration** — Fast transcription on Apple Silicon
- **Global Hotkey** — Works anywhere, even in fullscreen apps
- **Left/Right Key Detection** — Use RightOption, LeftCmd, etc. as single-key shortcuts
- **Auto-paste** — Types directly into focused input, or copies to clipboard if none
- **Multi-language** — Supports Korean, English, Japanese, Chinese, and more
- **Custom Vocabulary** — Help Whisper recognize technical terms like "Supabase", "Claude Code", etc.

## Features (Added)

### Claude Code Integration

Two output modes for seamless AI-assisted development:

| Mode | Description |
|------|-------------|
| **Direct Paste** | Pastes transcribed text at cursor position via Cmd+V (default) |
| **Print Mode** | Sends transcription to Claude Code CLI via `claude -p` |

### 4-Tab Settings UI

| Tab | Contents |
|-----|----------|
| **General** | Model selection (Official / Distil-Whisper / Quantized), language |
| **Recording** | Shortcut, recording mode (Toggle / Push-to-Talk), max duration |
| **Claude** | Output mode selector (Direct Paste / Print Mode) |
| **Advanced** | Custom vocabulary (comma-separated terms) |

### Recording Timer

- Configurable max recording duration (10–600 seconds)
- Automatic stop on timeout

### Media Key Toggle

- AirPods / EarPods play/pause button to toggle recording
- Implemented via macOS `CGEventTap` raw FFI

## Models

| Model | Size | Speed | Accuracy |
|-------|------|-------|----------|
| tiny | 75MB | Fastest | Basic |
| base | 142MB | Fast | Good |
| small | 466MB | Medium | Better |
| medium | 1.5GB | Slow | Great |
| large-v3 | 3GB | Slowest | Best |
| large-v3-turbo | 1.6GB | Medium | Great |

Download models directly from the app. Start with `base` or `small`.

## Requirements

- macOS 10.15+
- Apple Silicon recommended (Metal acceleration)
- Microphone permission
- Accessibility permission (for typing text)

## Build

```bash
# Install dependencies
npm install

# Run in development
npm run tauri dev

# Build for production
npm run tauri build
```

## Tech Stack

| Layer | Technology |
|-------|-----------|
| App Framework | [Tauri 2](https://tauri.app/) (Rust) |
| Frontend | [SvelteKit 5](https://kit.svelte.dev/) + TypeScript |
| Speech-to-Text | [whisper-rs](https://codeberg.org/tazz4843/whisper-rs) 0.15 (Metal) |
| Audio Capture | [cpal](https://github.com/RustAudio/cpal) 0.15 |
| Native Integration | Swift floating indicator, Core Graphics CGEventTap |
| Build | Vite 6 |

## Credits

Forked from [team-attention/nobs-whisper](https://github.com/team-attention/nobs-whisper) — thank you for the excellent foundation.

## License

MIT

Copyright (c) 2025 team-attention (original)
Copyright (c) 2025 SHINJUNGSUN (modifications)
