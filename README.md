# WindowedClaude

**Claude Code on Windows shouldn't be hard.** WindowedClaude wraps every install step into a single executable — no PowerShell, no PATH configs, no Git installs. Double-click, and you're in.

Built in Rust. Lightweight. Themed. Fast. For Windows and Mac coders.

---

## What It Does

WindowedClaude is a themed terminal window that:

- **One-click install** — downloads Git for Windows + Claude CLI automatically on first run
- **Multi-tab terminal** — run multiple Claude sessions side by side (`Ctrl+N`)
- **8 polished themes** — full ANSI 16-color palettes with canonical color values
- **Settings panel** — in-app GUI to change theme, font size, transparency, and reinstall shortcuts
- **Keybinds editor** — view, rebind, save, discard, or reset all 12 keyboard shortcuts from the UI
- **Interactive hover & click feedback** — settings/keybinds buttons highlight on hover and flash on click
- **Fully rounded window corners** — 12px transparent pixel masking for a modern look
- **Window transparency** — adjustable opacity so you can see through while working
- **Auto-accept mode** — dedicated Desktop + Start Menu shortcuts to run with `--dangerously-skip-permissions`
- **Shortcut icons** — all `.lnk` shortcuts show the WindowedClaude app icon
- **Clean uninstall** — `--uninstall` removes all traces (shortcuts, registry, data, self-deletes)
- **Add/Remove Programs** — registered in Windows Settings > Apps for easy discovery
- **Code signing ready** — self-signed cert script reduces SmartScreen warnings
- **Windows app manifest** — proper DPI awareness, UAC level, and OS compatibility declarations
- **Cross-platform** — runs on Windows (ConPTY) and macOS (Unix PTY)

No terminal experience needed. No admin rights needed.

---

## Quick Start

### Option 1: Download the Release

1. Go to [Releases](https://github.com/aaf2tbz/WindowedClaude/releases)
2. Download `windowed-claude.exe` (Windows) or `WindowedClaude-macos` (macOS ARM64)
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
rustup target add x86_64-pc-windows-gnu
cargo build --release --target x86_64-pc-windows-gnu
```

### Sign the Exe (Optional — Reduces SmartScreen Warnings)

```powershell
# On Windows, after building:
.\scripts\sign.ps1
```

Creates a self-signed code signing certificate and signs the exe with SHA256 + DigiCert timestamp. The publisher shows "WindowedClaude" instead of "Unknown" in Windows dialogs.

---

## Keyboard Shortcuts

### Tabs

| Shortcut | Action |
|----------|--------|
| `Ctrl+N` | New tab |
| `Ctrl+W` | Close current tab (last tab closes window) |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+1-9` | Jump to tab by number |

### Window & Display

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+O` | Toggle window transparency |
| `Ctrl+Shift+=` | Increase opacity |
| `Ctrl+Shift+-` | Decrease opacity |
| `Ctrl+=` | Increase font size |
| `Ctrl+-` | Decrease font size |
| `Ctrl+0` | Reset font size to default (14pt) |
| `Escape` | Close settings panel |

### Clipboard

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift+C` | Copy selected text |
| `Ctrl+Shift+V` | Paste from clipboard (bracket paste mode) |

You can also click the **theme pill** to cycle themes or **settings pill** to open the settings panel — both in the title bar.

---

## Multi-Tab Terminal

Open multiple Claude sessions in one window:

- **New tab**: `Ctrl+N` — spawns a new Claude instance in its own PTY
- **Close tab**: `Ctrl+W` — closing the last tab closes the window
- **Switch tabs**: Click a tab, `Ctrl+Tab` / `Ctrl+Shift+Tab`, or `Ctrl+1-9`
- Tab bar appears automatically when you have 2+ tabs
- Active tab highlighted with cursor accent color + bottom accent line
- Each tab has an X close button
- All tabs drain PTY output every frame (no memory buildup in background tabs)
- Window resize propagates to all tabs

---

## Settings Panel

Click the **Settings** pill in the title bar to open. Press `Escape`, click `X`, or click outside to close.

All interactive elements have **hover highlighting** (accent color background) and **click flash feedback** so you know your click registered.

| Setting | Control | Description |
|---------|---------|-------------|
| **Theme** | Click pill to cycle | Cycles through all 8 built-in themes |
| **Font Size** | `-` / `+` buttons | Range: 8pt to 48pt, persisted to config |
| **Transparency** | Click to toggle | Enables/disables window transparency |
| **Opacity** | `-` / `+` buttons | 5% to 100%, only visible when transparency is on |
| **Reinstall Shortcuts** | Click button | Recreates Desktop + Start Menu shortcuts with icons |

---

## Keybinds Editor

Click the **Keybinds** pill in the title bar to open the keybind configuration overlay.

**Layout:** Three-column table showing Action, Default value, and Current value for all 12 configurable shortcuts.

**How to rebind:**
1. Click any row — it enters key capture mode ("Press keys...")
2. Press your desired key combo (e.g., `Ctrl+Shift+K`)
3. The new binding appears in the Current column, highlighted in accent color if it differs from the default
4. Press `Escape` to cancel editing without changing

**Bottom buttons:**
- **Save** — persists all changes to `config.json`
- **Discard** — reverts to last saved state (undoes unsaved edits)
- **Reset to Defaults** — restores all 12 keybinds to factory values

**Configurable actions:**

| Action | Default |
|--------|---------|
| New Tab | `Ctrl+N` |
| Close Tab | `Ctrl+W` |
| Next Tab | `Ctrl+Tab` |
| Prev Tab | `Ctrl+Shift+Tab` |
| Toggle Transparency | `Ctrl+Shift+O` |
| Copy | `Ctrl+Shift+C` |
| Paste | `Ctrl+Shift+V` |
| Increase Opacity | `Ctrl+Shift+=` |
| Decrease Opacity | `Ctrl+Shift+-` |
| Font Size + | `Ctrl+=` |
| Font Size - | `Ctrl+-` |
| Reset Font | `Ctrl+0` |

---

## Themes

8 built-in themes, each with full ANSI 16-color palettes using canonical color values:

| Theme | Style |
|-------|-------|
| **Claude Dark** | Default. Warm amber accent on near-black |
| **Claude Light** | Clean light background with warm tones |
| **Midnight** | Deep blue-black with bright, high-contrast accents |
| **Solarized Dark** | Canonical Ethan Schoonover palette (proper bright color mapping) |
| **Dracula** | Purple-tinted dark theme with proper blue/purple distinction |
| **Nord** | Arctic blue-grey with differentiated bright variants |
| **Monokai Pro** | Warm dark with vivid, distinct blue vs cyan |
| **Gruvbox Dark** | Retro groove with refined earthy tones (bg4 bright black) |

Each theme defines:
- Custom title bar background + text color
- Terminal background + foreground
- Cursor accent color (used for UI highlights throughout)
- Selection highlight (background + foreground)
- Window border/padding color (distinct from background for depth)
- Full ANSI 16-color palette (8 normal + 8 bright, all canonical)

---

## Auto-Accept Mode

Launch Claude with `--dangerously-skip-permissions` to skip permission prompts for every tool call.

**Three ways to use it:**

1. **Desktop shortcut**: "WindowedClaude (Auto-Accept)" — created automatically with Desktop shortcuts
2. **Start Menu shortcut**: "WindowedClaude (Auto-Accept)" — always created on install
3. **Command line**: `windowed-claude --auto-accept`

The title bar shows `| AUTO` when running in this mode.

---

## Uninstall

To completely remove WindowedClaude and all its components:

```bash
windowed-claude --uninstall
```

This removes:
- Desktop shortcuts (main + auto-accept)
- Start Menu shortcuts (main + auto-accept)
- Right-click context menu registry entries
- Add/Remove Programs registry entry
- Configuration directory (`%APPDATA%\windowed-claude\`)
- Data directory (`%LOCALAPPDATA%\windowed-claude\`)
- Claude CLI — only if WindowedClaude installed it (checked via marker file)
- Git for Windows — only if WindowedClaude installed it (checked via marker file)
- The exe itself (delayed self-delete via `cmd /c`)

WindowedClaude registers with **Add/Remove Programs** during install, so you can also find it in Windows Settings > Apps > Installed apps.

---

## Window Features

- **Fully rounded corners** — 12px radius, transparent pixel masking on every frame
- **Custom title bar** — drag to move the window, double-click to maximize
- **Traffic light buttons** — close (red), maximize (yellow), minimize (green)
- **Theme pill** — click to cycle themes, shows current theme name
- **Settings pill** — click to open/close the settings overlay
- **Edge resize** — drag any edge or corner (5px detection zone)
- **Themed padding frame** — border color distinct from background for visual depth
- **Mouse text selection** — click and drag in terminal area to select text
- **Scrollback** — mouse wheel scrolls terminal history
- **Transparency** — window background becomes see-through, content remains readable
- **No decorations** — custom window chrome, no OS title bar

---

## First Run (Windows)

On the very first launch, WindowedClaude:

1. Opens the window immediately with a progress screen
2. Downloads **Git for Windows** (~50MB) from GitHub releases
3. Installs Git silently (`/VERYSILENT /NORESTART`)
4. Installs **Claude Code CLI** via PowerShell (`irm https://claude.ai/install.ps1 | iex`)
5. Creates **Start Menu shortcuts** (main + auto-accept)
6. Registers the **right-click context menu** ("Run with Auto-Accept")
7. Registers with **Add/Remove Programs**
8. Tracks what it installed (marker files for clean uninstall)
9. Shows a **welcome screen** asking if you want Desktop shortcuts
10. Spawns Claude in a real ConPTY terminal

Subsequent launches skip straight to Claude (< 1 second).

---

## Configuration

Settings are automatically persisted to disk and loaded on startup:

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

| Setting | Range | Description |
|---------|-------|-------------|
| `font_size` | 8.0 - 48.0 | Font size in points |
| `font_family` | — | Font name (JetBrains Mono embedded, not configurable yet) |
| `theme_id` | See themes | `claude-dark`, `claude-light`, `midnight`, `solarized-dark`, `dracula`, `nord`, `monokai`, `gruvbox` |
| `opacity` | 0.05 - 1.0 | Window opacity (only applies when `transparent` is true) |
| `transparent` | true/false | Enable window transparency |
| `git_bash_path` | path or null | Override Git Bash path (null = auto-detect standard locations + PATH) |

All settings can be changed via the Settings panel or keyboard shortcuts and are saved immediately.

---

## Code Signing

WindowedClaude includes a PowerShell script to sign the exe with a self-signed certificate:

```powershell
cargo build --release
.\scripts\sign.ps1
```

**What the script does:**
- Creates a self-signed code signing certificate (RSA 2048, SHA256, valid 5 years)
- Stores it in `Cert:\CurrentUser\My` (reused across builds)
- Signs the exe with Authenticode + DigiCert timestamp
- Exports the public cert to `assets/WindowedClaude.cer`

**What this gives you:**
- Publisher shows "WindowedClaude" instead of "Unknown" in Windows dialogs
- Consistent signing identity builds SmartScreen reputation faster
- Exe is tamper-evident (signature breaks if modified)
- Users can optionally trust the cert: `Import-Certificate -FilePath WindowedClaude.cer -CertStoreLocation Cert:\CurrentUser\Root`

For full SmartScreen bypass, use an OV code signing certificate from a CA like Certum, Sectigo, or SignPath.

---

## Tech Stack

| Component | Technology | Purpose |
|-----------|-----------|---------|
| Language | Rust | Performance, safety, cross-compilation |
| Window | winit 0.30 | Cross-platform window management |
| Pixel buffer | softbuffer 0.4 | Software rendering (no GPU required) |
| Font rendering | fontdue 0.9 | Pure Rust font rasterization |
| Terminal emulation | alacritty_terminal 0.25 | VT100/xterm-256color emulation |
| PTY | portable-pty 0.8 | ConPTY (Windows) / Unix PTY (macOS) |
| Clipboard | arboard 3 | System clipboard integration |
| Font | JetBrains Mono | Embedded TTF, no system font dependency |
| HTTP | reqwest 0.12 | Downloading Git installer |
| Serialization | serde + serde_json | Config file persistence |
| Registry | winreg 0.55 | Windows shortcuts + context menu |
| Async | tokio 1 | Background installer thread |

No OpenGL. No GPU. No Electron. Just pixels.

---

## Project Structure

```
src/
  main.rs              # Entry point, CLI args (--auto-accept, --uninstall)
  config.rs            # Settings persistence (JSON, auto-save)
  installer/
    mod.rs             # First-run orchestration + progress reporting
    git.rs             # Git for Windows download + silent install
    claude.rs          # Claude CLI installation (PowerShell/bash)
    shortcuts.rs       # .lnk shortcuts (with icons) + context menu
    uninstall.rs       # Clean uninstall (--uninstall)
  terminal/
    mod.rs             # alacritty_terminal wrapper (Arc<Mutex<Term>>)
    pty.rs             # PTY session (ConPTY/Unix, I/O threads)
  ui/
    mod.rs
    window.rs          # Event loop, input, multi-tab, settings, app state machine
    renderer.rs        # Software renderer (glyphs, cells, title bar, settings overlay, corner masking)
    theme.rs           # 8 built-in themes + Color type + ANSI 256-color support
assets/
  fonts/               # JetBrains Mono Regular (embedded at compile time)
  icon.ico             # Windows app icon (embedded via winresource)
  app.manifest         # Windows manifest (DPI, UAC asInvoker, Win10/11 compat)
scripts/
  sign.ps1             # Self-signed code signing (Authenticode + timestamp)
build.rs               # Windows resource embedding (icon, version info, manifest)
```

---

## CLI Flags

| Flag | Description |
|------|-------------|
| `--auto-accept` | Launch Claude with `--dangerously-skip-permissions` |
| `--uninstall` | Remove all WindowedClaude traces and self-delete |

---

## License

MIT
