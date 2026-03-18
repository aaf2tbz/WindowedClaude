# Contributing to WindowedClaude

Thanks for your interest in contributing! WindowedClaude is a themed terminal wrapper for Claude Code, built in Rust.

## Ground Rules

- **PRs are welcome**, but **direct merges to `master` are not allowed**. All changes go through pull request review.
- Keep PRs focused. One feature or fix per PR.
- Test your changes on Windows if possible — that's the primary target platform.
- Don't break the build. CI must pass before a PR is reviewed.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable toolchain)
- Git

### Build

```bash
git clone https://github.com/aaf2tbz/WindowedClaude.git
cd WindowedClaude
cargo build
```

### Run

```bash
cargo run                    # Normal mode
cargo run -- --auto-accept   # Auto-accept mode
RUST_LOG=info cargo run      # With debug logging
```

### Test on Windows

The primary target is Windows. If you're on macOS/Linux, the app still compiles and runs (it skips the Git installer and uses system bash), but the full experience requires Windows.

To cross-compile for Windows:

```bash
rustup target add x86_64-pc-windows-msvc
cargo build --release --target x86_64-pc-windows-msvc
```

## What to Contribute

Good first contributions:

- **New themes** — Add to `src/ui/theme.rs`. Follow the existing pattern: full ANSI 16-color palette + chrome colors.
- **Bug fixes** — Especially around terminal rendering edge cases.
- **Font support** — Adding bold/italic font variants.
- **Accessibility** — Screen reader support, high-contrast themes.

Larger contributions (discuss in an issue first):

- New rendering backends
- Installer changes
- Windows-specific shell integration
- New UI elements in the title bar

## Project Structure

```
src/
  main.rs              # Entry point, CLI args
  config.rs            # Settings (JSON persistence)
  installer/           # First-run setup (Git + Claude CLI)
  terminal/            # alacritty_terminal + PTY management
  ui/
    window.rs          # Event loop, input, window chrome
    renderer.rs        # Pixel buffer rendering (softbuffer + fontdue)
    theme.rs           # Color themes
```

## Adding a Theme

1. Open `src/ui/theme.rs`
2. Add a new `pub const YOUR_THEME: Theme = Theme { ... };`
3. Add the ID to `THEME_IDS` array
4. Add the match arm in `theme_by_id()`
5. Test: `cargo run`, then `Ctrl+Shift+T` to cycle to your theme

Each theme needs:
- `id` and `name`
- Title bar colors (`title_bar_bg`, `title_bar_text`, `window_border`)
- Terminal colors (`bg`, `fg`, `cursor`, `selection_bg`, `selection_fg`)
- Full ANSI 16-color array (`ansi: [Color; 16]`)

## Code Style

- Standard `cargo fmt` formatting
- No `unsafe` unless absolutely necessary (and documented)
- Prefer simple, readable code over clever abstractions
- Comments for non-obvious logic, not for self-documenting code

## Pull Request Process

1. Fork the repo
2. Create a feature branch (`git checkout -b my-feature`)
3. Make your changes
4. Run `cargo build` and `cargo clippy` — fix any warnings
5. Push to your fork
6. Open a PR against `master`
7. Wait for CI to pass and review

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
