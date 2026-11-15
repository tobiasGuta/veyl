# Architecture (v0.1)

## Process and widget model

```text
Gtk.Application (single application id)
  └─ Gtk.ApplicationWindow + Gtk.HeaderBar (custom CSS)
      ├─ tab bar: Gtk.Box containing selectable tab chips
      ├─ appearance Gtk.Popover (opacity setting)
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

GTK alpha-composited backgrounds are not equivalent to background blur. The source uses CSS background opacity, while VTE is told not to paint a solid background. Text remains opaque. On some compositor/theme combinations, actual transparency may differ; provide an opaque fallback and **test it**.

## Future abstraction boundaries

- Extract TerminalSession from UI model before split panes.
- Add shell/profile model only after lifecycle coverage.
- Add a compositor-capability interface only after a proven Wayland integration path.
- Unit test non-GUI configuration; use desktop integration testing for PTY/GTK behavior.
