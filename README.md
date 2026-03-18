# WindowedClaude

**Claude Code on Windows shouldn't be hard.** WindowedClaude wraps every install step into a single executable — no PowerShell, no PATH configs, no Git installs. Double-click, and you're in.

Built in Rust. Lightweight. Themed. Fast.

---

## What It Does

WindowedClaude is a themed terminal window that:

- **Downloads and installs everything** on first run (Git for Windows + Claude CLI)
- **Creates shortcuts** (Start Menu automatically, Desktop optionally)
- **Hosts Claude Code** inside a custom-rendered terminal with real VT100 emulation
- **Offers 8 polished themes** with full ANSI color palettes
- **Multi-tab support** — run multiple Claude sessions side by side
- **Settings panel** — change theme, font size, transparency, and reinstall shortcuts
- **Fully rounded window corners** for a modern look
- **Supports transparency** so you can see through the window while working
- **Adds a right-click option** to run Claude with `--dangerously-skip-permissions`
- **Clean uninstall** — `--uninstall` flag removes all traces (shortcuts, registry, data)

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

### Sign the Exe (Optional — Reduces SmartScreen Warnings)

```powershell
# On Windows, after building:
.\scripts\sign.ps1
```

Creates a self-signed code signing certificate and signs the exe with SHA256 + timestamp. The publisher will show "WindowedClaude" instead of "Unknown" in Windows dialogs.

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
| `Ctrl+N` | New tab |
| `Ctrl+W` | Close current tab (last tab closes window) |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+1-9` | Jump to tab by number |
| `Ctrl+Shift+O` | Toggle window transparency |
| `Ctrl+Shift+=` | Increase opacity |
| `Ctrl+Shift+-` | Decrease opacity |
| `Ctrl+Shift+C` | Copy selected text |
| `Ctrl+Shift+V` | Paste from clipboard |
| `Ctrl+=` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size to default (14pt) |
| `Escape` | Close settings panel |

You can also click the **theme pill** or **settings pill** in the title bar.

---

## Themes

8 built-in themes, each with full ANSI 16-color palettes:

| Theme | Style |
|-------|-------|
| **Claude Dark** | Default. Warm amber accent on near-black |
| **Claude Light** | Clean light background with warm tones |
| **Midnight** | Deep blue-black with bright, high-contrast accents |
| **Solarized Dark** | Canonical Ethan Schoonover palette |
| **Dracula** | Purple-tinted dark theme with proper blue/purple distinction |
| **Nord** | Arctic blue-grey with differentiated bright colors |
| **Monokai Pro** | Warm dark with vivid, distinct syntax colors |
| **Gruvbox Dark** | Retro groove with refined earthy tones |

All themes include:
- Custom title bar colors
- Terminal background + foreground
- Cursor accent color
- Selection highlight
- Window border/padding color
- Full ANSI 16-color mapping (canonical values)

---

## Multi-Tab Terminal

Open multiple Claude sessions in one window:

- **New tab**: `Ctrl+N`
- **Close tab**: `Ctrl+W` (closing the last tab closes the window)
- **Switch tabs**: Click on a tab, `Ctrl+Tab`, or `Ctrl+1-9`
- Tab bar appears automatically when you have 2+ tabs
- Each tab gets its own independent PTY session
- All tabs drain output every frame (no memory buildup in background tabs)

---

## Settings Panel

Click the **Settings** pill in the title bar (or press Escape to close):

- **Theme** — click to cycle through all 8 themes
- **Font Size** — +/- buttons (8pt to 48pt)
- **Transparency** — toggle on/off
- **Opacity** — +/- when transparency is on
- **Reinstall Shortcuts** — recreate Desktop + Start Menu shortcuts

---

## Auto-Accept Mode

Right-click the app (or shortcut) and select **"Run with Auto-Accept"** to launch Claude with `--dangerously-skip-permissions`. This skips the permission prompts for every tool call.

You can also run it from the command line:

```bash
windowed-claude --auto-accept
```

The title bar shows `| AUTO` when running in this mode.

---

## Uninstall

To completely remove WindowedClaude and all its components:

```bash
windowed-claude --uninstall
```

This removes:
- Desktop and Start Menu shortcuts
- Right-click context menu entries
- Add/Remove Programs registry entry
- Configuration and data directories
- Claude CLI (only if WindowedClaude installed it)
- Git for Windows (only if WindowedClaude installed it)
- The exe itself (delayed self-delete)

WindowedClaude also registers with **Add/Remove Programs** during install, so you can find it in Windows Settings > Apps.

---

## Window Features

- **Fully rounded corners** (12px radius, transparent masking)
- **Custom title bar** with drag-to-move
- **Traffic light buttons** (close, maximize, minimize)
- **Theme pill** + **Settings pill** in title bar
- **Edge resize** (drag any edge or corner to resize)
- **Themed padding** (border color matches theme)
- **Mouse text selection** (click and drag to select, Ctrl+Shift+C to copy)
- **Scroll** (mouse wheel scrolls terminal history)
- **Transparency** (toggle with Ctrl+Shift+O, adjust with Ctrl+Shift+=/-)

---

## First Run (Windows)

On the very first launch, WindowedClaude:

1. Downloads **Git for Windows** from GitHub
2. Installs **Claude Code CLI** via the official installer
3. Creates a **Start Menu shortcut**
4. Registers the **right-click context menu** option
5. Registers with **Add/Remove Programs**
6. Shows a **welcome screen** asking if you want a Desktop shortcut
7. Launches Claude

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

## Code Signing

WindowedClaude includes a self-signed code signing script to reduce Windows SmartScreen warnings:

```powershell
# Build release
cargo build --release

# Sign the exe
.\scripts\sign.ps1
```

The script:
- Creates a self-signed code signing certificate (persists in your cert store)
- Signs the exe with SHA256 + DigiCert timestamp
- Exports the public certificate to `assets/WindowedClaude.cer`

This gives the exe a consistent "WindowedClaude" publisher identity and makes SmartScreen less aggressive. For full SmartScreen bypass, consider an OV code signing certificate from a trusted CA.

---

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Language | Rust |
| Window | winit 0.30 |
| Pixel buffer | softbuffer 0.4 |
| Font rendering | fontdue 0.9 (pure Rust, no system deps) |
| Terminal emulation | alacritty_terminal 0.25 |
| PTY | portable-pty 0.8 (ConPTY on Windows) |
| Clipboard | arboard 3 |
| Font | JetBrains Mono (embedded) |

No OpenGL. No GPU. No Electron. Just pixels.

---

## Project Structure

```
src/
  main.rs              # Entry point, CLI arg parsing (--auto-accept, --uninstall)
  config.rs            # Settings persistence (JSON)
  installer/
    mod.rs             # First-run orchestration
    git.rs             # Git for Windows download + installation
    claude.rs          # Claude CLI installation
    shortcuts.rs       # Shortcuts + context menu registration
    uninstall.rs       # Clean uninstall (--uninstall)
  terminal/
    mod.rs             # alacritty_terminal integration
    pty.rs             # PTY session (ConPTY/Unix)
  ui/
    mod.rs
    window.rs          # winit event loop, input, multi-tab, settings
    renderer.rs        # Software renderer (cells, glyphs, title bar, settings overlay)
    theme.rs           # 8 built-in themes + color utilities
assets/
  fonts/               # Embedded JetBrains Mono
  icon.ico             # App icon
  app.manifest         # Windows application manifest (UAC, DPI, compatibility)
scripts/
  sign.ps1             # Self-signed code signing script
```

---

## License

MIT
