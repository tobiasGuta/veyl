# Architecture (v0.1)

## Process and widget model

```text
Gtk.Application (single application id)
  └─ Gtk.ApplicationWindow + Gtk.HeaderBar (custom CSS)
      ├─ tab bar: Gtk.Box containing selectable tab chips
      ├─ appearance Gtk.Popover (blur and opacity)
      └─ Gtk.Stack: one surface + Vte.Terminal per tab
                      │
                      └─ VTE PTY → $SHELL (or /bin/bash)
                                        ├─ editor, ssh, tmux, ...
                                        └─ child processes
```

The terminal engine is upstream VTE4, **not a new ANSI/VT parser**. GTK4 renders the shell viewport and owns Wayland client-side window decorations. The application uses no privileged helper.

## Responsibilities

- src/main.rs: register GTK app id and activation callback.
- src/window.rs: window/header/tab state, VTE widgets, PTY spawning, appearance controls, shortcuts.
- src/config.rs: defaults, normalization, user configuration load/save and unit tests.
- assets/veyl.css: static design tokens and widget appearance.
- docs/: product contracts, acceptance tests, unsupported cases, agent instructions.

## State and lifecycle

One process hosts multiple sessions. Each tab has a unique numeric identifier, GTK surface and VTE Terminal widget, all owned by State. Switching tabs changes the visible Stack child. Tab close removes its GTK widgets; VTE closes its PTY, which normally causes a SIGHUP to interactive foreground processes. Do not assume all descendants terminate. **Codex priority:** audit reentrant GTK signal handlers, ownership cycles, failure handling, and process cleanup under close and shutdown.

VTE spawns the user's `$SHELL` as an executable path, **not** via `sh -c`. It inherits the process environment and starts in `$HOME`; input typed into the terminal executes with the user's normal permissions. Do not setuid, wrap in `sudo`, or automatically elevate.

## Configuration

Path resolution: `$XDG_CONFIG_HOME/veyl/config.toml` else `$HOME/.config/veyl/config.toml`. Use serde for TOML fields; clamp unreasonable numeric values. Slider saves config to the same file. Settings are scoped to a single user; no global system config is written.

## Rendering constraint

The GTK window paints one background tint beneath the header and VTE. VTE skips clearing its default background, leaving glyphs and terminal color cells opaque. On GTK 4.24+ with `ext-background-effect-v1` advertised by the Wayland display, GTK sends the blur region for the window's backdrop filter to the compositor. When that path is unavailable, the default frosted setting uses an opaque tint. Plain background alpha is available by turning off frosted mode.

## Future abstraction boundaries

- Extract TerminalSession from UI model before split panes.
- Add shell/profile model only after lifecycle coverage.
- Unit test non-GUI configuration; use desktop integration testing for PTY/GTK behavior.
