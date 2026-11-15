# Veyl

**Veyl** is an early-stage, Fedora-native terminal emulator with a visual direction inspired by **Windows Terminal on Windows 11**: compact integrated tabs, a dark Fluent-like surface, and subtle translucency.

> **Status:** v0.1 development foundation; not a production-ready daily-driver, and not yet verified on a real Fedora 45 GNOME Wayland desktop. Real background blur is **not implemented**. See [STATUS.md](docs/STATUS.md).

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
- Appearance popover with a per-user **background opacity** slider. The configuration is saved at `$XDG_CONFIG_HOME/veyl/config.toml` or `~/.config/veyl/config.toml`.
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

The initial target is *subtle translucency*, not full see-through transparency. The initial opacity is **0.86**, editable from the dropdown. Veyl uses a semi-transparent GTK background while keeping text opaque; it deliberately does **not** lower the opacity of the entire window.

**Acrylic/Mica-style background blur is not part of v0.1.** GNOME Wayland does not expose the same compositor API as Windows. Blur My Shell or other extensions may affect window blur depending on versions and settings, but Veyl must neither require extensions nor claim an exact effect without testing. See [Visual spec](docs/VISUAL_SPEC.md).

## Configuration

Example `~/.config/veyl/config.toml`:

~~~toml
opacity = 0.86
font_family = "Cascadia Mono"
font_size = 12
scrollback_lines = 10000
background = "#10121a"
foreground = "#e8eaf0"
~~~

Create the file manually to override defaults; opening the opacity slider and changing it also creates the file. Other settings take effect at application launch, with live opacity updates only in v0.1. Invalid configuration falls back to defaults, with a message on stderr.

## For Codex contributors

**Start with [AGENTS.md](AGENTS.md)**, then [Codex handoff](docs/CODEX_HANDOFF.md), [Architecture](docs/ARCHITECTURE.md), [Visual spec](docs/VISUAL_SPEC.md), [Testing](docs/TESTING.md), [Security](docs/SECURITY.md), and [Roadmap](docs/ROADMAP.md). Explicit acceptance criteria and non-goals are documented so agents don't fabricate untested features.

## Project identity and attribution

Veyl is an independent Linux project, not affiliated with Microsoft. “Windows Terminal”, “Fluent”, “Acrylic”, and “Mica” refer to external design inspiration and platform features, not Microsoft code or bundled graphics. Do not import Microsoft branding, proprietary assets, or copyrighted screenshots into the app.

No license has been selected by the repository owner yet. Do not infer permission to relicense the project; resolve that decision before publishing binary releases.
