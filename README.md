# WindowedClaude

**Claude Code on Windows shouldn't be hard.** WindowedClaude wraps every install step into a single executable — no PowerShell, no PATH configs, no Git installs. Double-click, and you're in.

Built in Rust. Lightweight. Themed. Fast.

---

## What It Does

WindowedClaude is a themed terminal window that:

- **Downloads and installs everything** on first run (Git for Windows + Claude CLI)
- **Creates shortcuts** (Start Menu automatically, Desktop optionally)
- **Hosts Claude Code** inside a custom-rendered terminal with real VT100 emulation
- **Offers 8 built-in themes** you can cycle through with a click or hotkey
- **Supports transparency** so you can see through the window while working
- **Adds a right-click option** to run Claude with `--dangerously-skip-permissions`

No terminal experience needed. No admin rights needed.

---

## Quick Start

### Option 1: Download the Release

1. Go to [Releases](https://github.com/aaf2tbz/WindowedClaude/releases)
2. Download `windowed-claude.exe`
3. Double-click it
4. Done

### Option 2: Build from Source

```bash
# Clone
git clone https://github.com/aaf2tbz/WindowedClaude.git
cd WindowedClaude

# Build (release)
cargo build --release

# Run
./target/release/windowed-claude      # macOS/Linux
.\target\release\windowed-claude.exe  # Windows
```

### Cross-compile for Windows from macOS/Linux

```bash
# Install the Windows target
rustup target add x86_64-pc-windows-msvc

# Build (requires cross or a Windows SDK)
cargo build --release --target x86_64-pc-windows-msvc
```

---

## Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+T` | Cycle through themes |
| `Ctrl+Shift+O` | Toggle window transparency |
| `Ctrl+Shift+=` | Increase opacity |
| `Ctrl+Shift+-` | Decrease opacity |
| `Ctrl+Shift+C` | Copy selected text |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+=` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size to default (14pt) |

You can also click the **theme pill** in the title bar to cycle themes.

---

## Themes

8 built-in themes, each with full ANSI 16-color palettes:

| Theme | Style |
|-------|-------|
| **Claude Dark** | Default. Warm amber accent on near-black |
| **Claude Light** | Clean light background with warm tones |
| **Midnight** | Deep blue-black with bright accents |
| **Solarized Dark** | Ethan Schoonover's classic dark palette |
| **Dracula** | Purple-tinted dark theme |
| **Nord** | Arctic, blue-grey palette |
| **Monokai Pro** | Warm dark with vivid syntax colors |
| **Gruvbox Dark** | Retro groove with earthy tones |

All themes include:
- Custom title bar colors
- Terminal background + foreground
- Cursor accent color
- Selection highlight
- Window border/padding color
- Full ANSI 16-color mapping

---

## Auto-Accept Mode

Right-click the app (or shortcut) and select **"Run with Auto-Accept"** to launch Claude with `--dangerously-skip-permissions`. This skips the permission prompts for every tool call.

You can also run it from the command line:

```bash
windowed-claude --auto-accept
```

The title bar shows `| AUTO` when running in this mode.

---

## Window Features

- **Custom title bar** with drag-to-move
- **Traffic light buttons** (close, maximize, minimize)
- **Edge resize** (drag any edge or corner to resize)
- **Rounded terminal corners** (10px radius)
- **Themed padding** (border color matches theme)
- **Mouse text selection** (click and drag to select, Ctrl+Shift+C to copy)
- **Scroll** (mouse wheel scrolls terminal history)
- **Transparency** (toggle with Ctrl+Shift+O, adjust with Ctrl+Shift+=/-)

---

## First Run (Windows)

On the very first launch, WindowedClaude:

1. Downloads **MinGit** (~45MB) from GitHub — a portable Git distribution
2. Installs **Claude Code CLI** via the official installer
3. Creates a **Start Menu shortcut**
4. Registers the **right-click context menu** option
5. Shows a **welcome screen** asking if you want a Desktop shortcut
6. Launches Claude

Subsequent launches skip straight to Claude (< 1 second).

---

## Configuration

Settings are persisted to:
- **Windows**: `%APPDATA%\windowed-claude\config.json`
- **macOS**: `~/Library/Application Support/windowed-claude/config.json`

```json
{
  "font_size": 14.0,
  "font_family": "JetBrains Mono",
  "theme_id": "claude-dark",
  "opacity": 1.0,
  "transparent": false,
  "git_bash_path": null
}
```

| Setting | Description |
|---------|-------------|
| `font_size` | Font size in points (8-48) |
| `font_family` | Font name (currently JetBrains Mono embedded) |
| `theme_id` | One of: `claude-dark`, `claude-light`, `midnight`, `solarized-dark`, `dracula`, `nord`, `monokai`, `gruvbox` |
| `opacity` | Window opacity 0.05 - 1.0 (only applies when `transparent` is true) |
| `transparent` | Enable window transparency |
| `git_bash_path` | Override path to Git Bash (null = auto-detect) |

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| Window | winit 0.30 |
| Pixel buffer | softbuffer 0.4 |
| Font rendering | fontdue 0.9 (pure Rust, no system deps) |
| Terminal emulation | alacritty_terminal 0.25 |
| Clipboard | arboard 3 |
| Font | JetBrains Mono (embedded) |

No OpenGL. No GPU. No Electron. Just pixels.

---

## Project Structure

```
src/
  main.rs              # Entry point, CLI arg parsing
  config.rs            # Settings persistence (JSON)
  installer/
    mod.rs             # First-run orchestration
    git.rs             # MinGit download + extraction
    claude.rs          # Claude CLI installation
    shortcuts.rs       # Shortcuts + context menu registration
  terminal/
    mod.rs             # alacritty_terminal integration
    pty.rs             # PTY session (Git Bash -> Claude)
  ui/
    mod.rs
    window.rs          # winit event loop, input handling
    renderer.rs        # Software renderer (cells, glyphs, title bar)
    theme.rs           # 8 built-in themes + color utilities
assets/
  fonts/               # Embedded JetBrains Mono
  icon.ico             # App icon
```

---

## License

MIT
