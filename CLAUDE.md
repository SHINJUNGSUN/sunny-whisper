# Sunny's Whisper

nobs-whisper 포크. Tauri 2 + SvelteKit 5 기반 macOS 로컬 음성 인식 앱.

## Architecture

- **Backend**: Tauri 2 (Rust) — 오디오 캡처, Whisper 추론, 텍스트 입력
- **Frontend**: SvelteKit 5 + TypeScript — 설정 UI, 인디케이터
- **Native**: Swift helper (NobsWhisperIndicator) — 플로팅 인디케이터
- **Config**: `~/.config/NobsWhisper/config.json`

## Key Paths

| 영역 | 파일 |
|------|------|
| Tauri 빌더 + 커맨드 등록 | `src-tauri/src/lib.rs` |
| AppConfig, ClaudeCodeMode | `src-tauri/src/config.rs` |
| 녹음 라이프사이클, 모드 분기 | `src-tauri/src/state.rs` |
| 텍스트 입력, send_to_claude_code | `src-tauri/src/input.rs` |
| 네이티브 단축키, 미디어 키 | `src-tauri/src/native_shortcut.rs` |
| 설정 UI (4탭) | `src/routes/+page.svelte` |
| 인디케이터 UI | `src/routes/indicator/+page.svelte` |
| 릴리스 워크플로우 | `.github/workflows/release.yml` |

## Build

```bash
npm run tauri dev       # 개발 실행
npm run tauri build     # 프로덕션 빌드
npm run check           # Svelte + TypeScript 체크
cd src-tauri && cargo check  # Rust 체크
```

## Conventions

- **Rust**: snake_case, `thiserror` 에러, `log` 크레이트
- **Svelte**: TypeScript strict, Svelte 5 runes (`$state`, `$derived`)
- **Tauri command 추가**: `src-tauri/src/lib.rs` invoke_handler에 등록 필수
- **Config field 추가**: `config.rs` AppConfig + `+page.svelte` UI 양쪽 수정
- **버전 변경**: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json` 3곳 동시 수정

## Git

| Remote | Repository |
|--------|-----------|
| origin | SHINJUNGSUN/sunny-whisper |
| upstream | team-attention/nobs-whisper |

Release: `git tag v*` → push → GitHub Actions → Universal DMG
