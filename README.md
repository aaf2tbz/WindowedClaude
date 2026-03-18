# WindowedClaude

A themed terminal for Claude Code. One exe, zero setup. Windows, macOS, and Linux.

Built in Rust. No GPU. No Electron. Just pixels.

---

## Download

Grab the latest from [Releases](https://github.com/aaf2tbz/WindowedClaude/releases):

| Platform | File | Notes |
|----------|------|-------|
| Windows x64 | `windowed-claude.exe` | Double-click and go |
| macOS (Universal) | `WindowedClaude-macos.zip` | Unzip, remove quarantine (see below) |
| Linux x64 | `WindowedClaude-linux.tar.gz` | Extract, then `./WindowedClaude` |

First launch auto-installs Claude CLI on all platforms (+ Git on Windows). No manual setup needed.

**macOS Gatekeeper:** macOS may flag the app as unidentified. After unzipping, run:
```bash
xattr -cr WindowedClaude
./WindowedClaude
```
Or: System Settings > Privacy & Security > click "Allow Anyway" after the first blocked launch.

---

## Features

**Terminal**
- Multi-tab sessions (`Ctrl+N` / `Ctrl+W` / `Ctrl+Tab`)
- Real PTY (ConPTY on Windows, Unix PTY on macOS/Linux)
- Full VT100/xterm-256color emulation via alacritty_terminal
- Mouse text selection + clipboard (`Ctrl+Shift+C/V`)
- Scrollback via mouse wheel

**Window**
- 9 polished themes with full ANSI 16-color palettes
- Rounded corners (12px transparent masking)
- Adjustable transparency + opacity
- Configurable padding (0-48px)
- Custom title bar with drag, resize, traffic light buttons
- Theme pill, Settings pill, Keybinds pill in title bar

**Settings Panel** (click "Settings" in title bar)
- Theme, font size, transparency, opacity, padding
- Reinstall shortcuts button
- Hover highlighting + click flash feedback

**Keybinds Editor** (click "Keybinds" in title bar)
- View and rebind all 12 keyboard shortcuts
- Shows default vs current values
- Save, Discard, Reset to Defaults
- Click a row, press your new combo, done

**Auto-Accept Mode**
- Dedicated shortcuts: "WindowedClaude (Auto-Accept)" on Desktop + Start Menu
- Or run: `windowed-claude --auto-accept`

**Uninstall** (Windows)
- `windowed-claude --uninstall` removes everything
- Also in Add/Remove Programs

---

## Keyboard Shortcuts

All shortcuts are rebindable via the Keybinds editor.

| Default | Action |
|---------|--------|
| `Ctrl+N` | New tab |
| `Ctrl+W` | Close tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+1-9` | Jump to tab |
| `Ctrl+Shift+O` | Toggle transparency |
| `Ctrl+Shift+=/-` | Adjust opacity |
| `Ctrl+Shift+C/V` | Copy / Paste |
| `Ctrl+=/-/0` | Font size +/-/reset |
| `Escape` | Close overlay |

---

## Themes

| Theme | Style |
|-------|-------|
| **Claude Dark** | Warm amber on near-black (default) |
| **Claude Light** | Clean light with warm tones |
| **Midnight** | Deep blue-black, high contrast |
| **Solarized Dark** | Canonical Schoonover palette |
| **Dracula** | Purple-tinted dark |
| **Nord** | Arctic blue-grey |
| **Monokai Pro** | Warm dark, vivid syntax |
| **Gruvbox Dark** | Retro earthy tones |
| **Developer** | Obsidian black, electric cyan accents |

---

## Security

**VirusTotal:** [1/72 — Clean](https://www.virustotal.com/gui/file/38d069d9d0a139f71176585a8446bff0b491da76a0e4e9eac1683a1916a10673/detection)

The single flag (Microsoft `Program:Win32/Wacapew.Clml`) is a generic heuristic false positive common with unsigned Rust executables. All 71 other engines report clean.

The source code is fully open. You can audit every line and build from source yourself.

---

## Build from Source

```bash
git clone https://github.com/aaf2tbz/WindowedClaude.git
cd WindowedClaude
cargo build --release
```

Cross-compile for Windows:
```bash
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

Sign the exe (optional, reduces SmartScreen warnings):
```powershell
.\scripts\sign.ps1
```

---

## Configuration

Settings auto-save to:
- **Windows**: `%APPDATA%\windowed-claude\config.json`
- **macOS**: `~/Library/Application Support/windowed-claude/config.json`
- **Linux**: `~/.config/windowed-claude/config.json`

```json
{
  "font_size": 14.0,
  "theme_id": "claude-dark",
  "opacity": 1.0,
  "transparent": false,
  "padding": 12,
  "keybinds": { ... }
}
```

All settings can be changed via the Settings panel or Keybinds editor.

---

## CLI Flags

| Flag | Description |
|------|-------------|
| `--auto-accept` | Skip Claude permission prompts |
| `--uninstall` | Remove all traces (Windows) |

---

## Tech Stack

Rust, winit, softbuffer, fontdue, alacritty_terminal, portable-pty, arboard. JetBrains Mono embedded.

---

## License

MIT
