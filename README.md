# Cozy Works

A cozy pixel-zen workspace desktop app built with **Tauri 2** + **Svelte 5**.

> Cozy pixel-zen workspace with Pomodoro, Todo/Kanban, Ambient sounds, Pet, Avatar, Notes, Memo, Terminal (with AI assistant), and Music player.

## Features

| Feature | Description |
|---|---|
| 🍅 Pomodoro | 25/5/15 (customizable), ring timer, daily stats, syncs with pet/avatar/ambient |
| ☑ Todo + Kanban | Tasks with priority/tag/due/repeat/subtasks, board (To Do/Doing/Done) + list/today/all views |
| 🌧 Ambient | Hybrid: synthesized rain/city via Web Audio + generated music pads for morning/afternoon/evening/night; auto follows time of day or manual |
| 🐾 Pet | Pixel-art companion — choose cat/rabbit/fox/dog/tanuki; reacts to focus, todo completion, night |
| 👤 Avatar | Pixel-art profile with mood (focus/break/night), customizable hair/skin color |
| ✎ Notes | Markdown editor with live preview, pin, archive, search, tags, [[wiki]] backlinks |
| ✧ Memo | Quick sticky notes |
| ▮ Terminal | Real PTY (bash/zsh/cmd via portable-pty) with xterm.js, git branch + dirty status bar, tabs |
| ✦ AI Assistant | Chat sidebar streaming to 9router (OpenAI-compatible), model picker, insert command → terminal, notifications on completion/confirmation |
| ♪ Music | Local folder scan (mp3/flac/wav/ogg/m4a), play/pause/next/prev, shuffle, repeat, volume |
| ⚙ Settings | All preferences + language EN/ID |

## Tech Stack

- **Tauri 2** (Rust backend, WebKit/WebView2 frontend container)
- **Svelte 5** + Vite (frontend)
- **portable-pty** + **xterm.js** (terminal)
- **tauri-plugin-sql** (SQLite: todos, notes, memos, pomodoro_stats, settings)
- **Web Audio API** (synthesized ambient — zero audio assets)

## Development

```bash
# Prereqs (Linux)
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev libsoup-3.0-dev libgtk-3-dev

# Install
npm install
cargo install tauri-cli --version 2.11.4 --locked

# Dev (hot reload)
npm run tauri dev

# Build
npm run tauri build
```

## Project Structure

```
cozy-works/
├── src/                    # Svelte 5 frontend
│   ├── App.svelte          # layout: sidebar + main + AI panel
│   ├── lib/
│   │   ├── stores.js       # global state (settings, pomodoro, pet, ambient, todos...)
│   │   ├── ambient.js      # Web Audio synthesis engine
│   │   ├── i18n.js         # EN/ID dictionaries
│   │   └── components/     # Pomodoro, Todo, Notes, Memo, Music, Terminal, AiPanel, Pet, Avatar, Settings, Dashboard...
└── src-tauri/              # Rust backend
    ├── src/lib.rs          # app entry, SQLite migrations, plugin wiring
    ├── src/pty.rs          # terminal PTY + git branch commands
    ├── src/music.rs        # music library scan
    └── tauri.conf.json
```

## AI Provider

The AI assistant uses an OpenAI-compatible API (default: your 9router endpoint `https://router.takora.dev/v1`). Configure model + base URL in **Settings → AI**. API key can be entered there (stored in local SQLite).

## Roadmap (v2)

- Notes wiki graph (backlinks visualization)
- Dashboard widget drag (desktop-style layout)
- AI tool-use / inline agent in terminal (direct execution)
- Internet radio
- TTS voice for AI
- Tray icon + global shortcuts
- Multi-window / workspace layouts

## License

Private project.
