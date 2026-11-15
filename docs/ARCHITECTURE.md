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

The window owns its shared state. The state holds a weak reference back to the window. Actions and tab callbacks also use weak references. Each tab has a unique ID, a GTK surface, a VTE terminal, and a cancellable for the shell launch. Switching tabs changes the visible Stack child.

Closing a tab removes it from the state before cancelling the launch and unparenting its widgets. Window teardown drains all tabs through the same path. GTK and VTE operations that can emit signals run after the state borrow ends, so a late child-exit signal sees the removed tab ID and returns. VTE disposes the terminal, signals the child, and releases the PTY. A short-lived application hold lets an in-flight launch finish or cancel when the last window closes.

When spawning fails in an open tab, Veyl displays the error in that tab and disables input. Closing the failed tab uses the ordinary close action.

VTE spawns the user's `$SHELL` as an executable path, **not** via `sh -c`. It inherits the process environment and starts in `$HOME`; input typed into the terminal executes with the user's normal permissions. Do not setuid, wrap in `sudo`, or automatically elevate.

## Configuration

Path resolution: `$XDG_CONFIG_HOME/veyl/config.toml` else `$HOME/.config/veyl/config.toml`. Use serde for TOML fields; clamp unreasonable numeric values. Slider saves config to the same file. Settings are scoped to a single user; no global system config is written.

## Rendering constraint

The GTK window paints one background tint beneath the header and VTE. VTE skips clearing its default background, leaving glyphs and terminal color cells opaque. On GTK 4.24+ with `ext-background-effect-v1` advertised by the Wayland display, GTK sends the blur region for the window's backdrop filter to the compositor. When that path is unavailable, the default frosted setting uses an opaque tint. Plain background alpha is available by turning off frosted mode.

Veyl checks for the protocol manager when it creates the window. Mutter 51.1 advertises the blur capability when a client binds the manager. The protocol permits capability changes, and GTK 4.24 does not expose those changes through the registry query. Compositors that withdraw blur while keeping the manager advertised can show a plain tint until the user changes the appearance setting.

## Future abstraction boundaries

- Extract TerminalSession from UI model before split panes.
- Add shell/profile model only after lifecycle coverage.
- Unit test non-GUI configuration; use desktop integration testing for PTY/GTK behavior.
