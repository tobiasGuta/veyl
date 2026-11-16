# Architecture (v0.1)

## Process and widget model

```text
Gtk.Application (single application id)
  └─ Gtk.ApplicationWindow + Gtk.HeaderBar (custom CSS)
      ├─ tab bar: Gtk.Box containing selectable tab chips
      ├─ tab menu Gtk.Popover (profiles and terminal actions)
      └─ Gtk.Stack: terminal tab surfaces and a Settings workspace
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

The window owns its shared state. The state holds a weak reference back to the window. Actions and callbacks also use weak references. A tab owns a GTK surface containing one pane or a tree of `GtkPaned` splits. Each pane has its own VTE terminal, search controls, and launch cancellable. Switching tabs changes the visible Stack child and restores focus to that tab's active pane.

Closing a tab removes it from the state before cancelling its panes' launches and unparenting its widgets. Closing an individual pane removes its model entry and promotes its sibling in the split tree. Closing or exiting the final pane closes its tab. Window teardown drains all tabs through the same path. GTK and VTE operations that can emit signals run after the state borrow ends, so a late child-exit signal sees the removed ID and returns. VTE disposes the terminal, signals the child, and releases the PTY. A short-lived application hold lets an in-flight launch finish or cancel when the last window closes.

When spawning fails in an open tab, Veyl displays the error in that tab and disables input. Closing the failed tab uses the ordinary close action.

VTE launches a profile executable or falls back to the user's `$SHELL`. A profile can select a starting folder, tab name, and ANSI scheme. Duplicate and split actions start a separate PTY with the same launch settings and a validated local starting folder. Input typed into the terminal executes with the user's normal permissions.

## Configuration

Path resolution: `$XDG_CONFIG_HOME/veyl/config.toml` else `$HOME/.config/veyl/config.toml`. Use serde for TOML fields; clamp unreasonable numeric values. Save in the Settings tab writes the changed fields and applies them to the open window. The draft lives with that tab, and its preview terminals have no PTY. Settings are scoped to a single user; no global system config is written.

Veyl launches Bash with a generated startup file when a custom prompt is enabled. The file sources the user's usual `.bashrc`, then registers a hook to set PS1 after existing prompt hooks. Veyl passes the PS1 value as data and clears its transport variable before sourcing the user file. The optional dotfile action independently updates a marked block in `.bashrc` using an atomic write.

## Rendering constraint

The GTK window paints one background tint beneath the header and VTE. VTE skips clearing its default background, leaving glyphs and terminal color cells opaque. On GTK 4.24+ with `ext-background-effect-v1` advertised by the Wayland display, GTK sends the blur region for the window's backdrop filter to the compositor. When that path is unavailable, the default frosted setting uses an opaque tint. Plain background alpha is available by turning off frosted mode.

Veyl checks for the protocol manager when it creates the window. Mutter 51.1 advertises the blur capability when a client binds the manager. The protocol permits capability changes, and GTK 4.24 does not expose those changes through the registry query. Compositors that withdraw blur while keeping the manager advertised can show a plain tint until the user changes the appearance setting.

## Future abstraction boundaries

- Unit test non-GUI configuration; use desktop integration testing for PTY/GTK behavior.
