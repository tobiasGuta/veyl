# Veyl

**Veyl** is an early-stage, Fedora-native terminal emulator with a visual direction inspired by **Windows Terminal on Windows 11**: compact integrated tabs, a dark Fluent-like surface, and subtle translucency.

> **Status:** v0.1 development foundation. See [verification status](docs/STATUS.md) for desktop coverage and remaining release gates.

## Quick start — Fedora 45

Install the native libraries and Rust toolchain:

~~~bash
sudo dnf install rust cargo gcc pkgconf-pkg-config gtk4-devel vte291-gtk4-devel
# Optional preferred font; fallback to another monospace font if unavailable:
sudo dnf install cascadia-mono-fonts
~~~

From the repository root:

~~~bash
cargo run
~~~

Veyl opens a GTK4 window and uses VTE4 to spawn the current user's shell using the Linux pseudo-terminal subsystem. It does **not** replace Bash, Fish, Zsh, or the system terminal. Keep your existing terminal installed while evaluating it.

If the application fails to compile, please report the exact `cargo build` output; the first independent Fedora validation is an explicit milestone.

## What v0.1 includes in source

- Rust + GTK4 application with custom dark header, integrated selectable tabs, new/close tab buttons and Wayland-friendly GTK window management.
- VTE terminal widget per tab, launching the `$SHELL` executable (or `/bin/bash` fallback), with regular shell environment inheritance.
- Windows Terminal-inspired styling in [assets/veyl.css](assets/veyl.css).
- Appearance popover with **Frosted background** and **Background opacity** controls. Veyl saves them in `$XDG_CONFIG_HOME/veyl/config.toml` or `~/.config/veyl/config.toml`.
- Configurable font, font size, foreground/background colors, and scrollback via TOML.
- Clipboard shortcuts and basic tab shortcuts.
- No telemetry, command-history collection, AI execution, or remote service integration.

### Keyboard shortcuts

| Shortcut | Intended action |
| --- | --- |
| Ctrl+Shift+T | New tab |
| Ctrl+Shift+W | Close active tab |
| Ctrl+Shift+C | Copy selection |
| Ctrl+Shift+V | Paste clipboard |

**Caution:** Closing a tab can terminate or disconnect interactive programs. Do not test with unsaved work.

## Appearance and transparency

The arrow beside **+** opens appearance settings. The default frosted background uses a **72%** dark tint over native compositor blur. The **Background opacity** slider changes the tint while keeping terminal text fully opaque.

Frosted mode uses GTK 4.24 and the Wayland `ext-background-effect-v1` protocol. Veyl uses a solid dark background when this path is unavailable. Turn **Frosted background** off to use plain translucency, or set opacity to **100%** for a solid background. See the [visual spec](docs/VISUAL_SPEC.md).

## Configuration

Example `~/.config/veyl/config.toml`:

~~~toml
opacity = 0.72
blur = true
font_family = "Cascadia Mono"
font_size = 12
scrollback_lines = 10000
background = "#10121a"
foreground = "#e8eaf0"
~~~

Create the file manually to override defaults. Changing an appearance control also creates the file. Blur and opacity update live, and the other TOML settings take effect at launch. Invalid configuration falls back to defaults with a message on stderr. Use [the window CSS selectors](docs/VISUAL_SPEC.md#customizing-the-window-chrome) to customize colors and spacing in the tab strip and frame.

## For Codex contributors

**Start with [AGENTS.md](AGENTS.md)**, then [Codex handoff](docs/CODEX_HANDOFF.md), [Architecture](docs/ARCHITECTURE.md), [Visual spec](docs/VISUAL_SPEC.md), [Testing](docs/TESTING.md), [Security](docs/SECURITY.md), and [Roadmap](docs/ROADMAP.md). Explicit acceptance criteria and non-goals are documented so agents don't fabricate untested features.

## Project identity and attribution

Veyl is an independent Linux project, not affiliated with Microsoft. “Windows Terminal”, “Fluent”, “Acrylic”, and “Mica” refer to external design inspiration and platform features, not Microsoft code or bundled graphics. Do not import Microsoft branding, proprietary assets, or copyrighted screenshots into the app.

No license has been selected by the repository owner yet. Do not infer permission to relicense the project; resolve that decision before publishing binary releases.
