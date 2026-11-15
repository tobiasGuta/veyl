# Roadmap

All items are proposed, not promises or shipped features. Preserve scope discipline.

## v0.1 — Native terminal foundation (source stage)
- Rust GTK4/VTE4 application on Fedora 45 GNOME Wayland.
- One independent shell/PTY per tab.
- Tabs (new/select/close) and baseline clipboard shortcuts.
- Windows Terminal-inspired dark compact header/tab aesthetic.
- Configurable background opacity without dimming text, font/colors via TOML.
- CI, docs, tests; runtime/visual/security validation outstanding.
- Completion requires acceptance criteria in CODEX_HANDOFF.md.

## v0.2 — Visual fidelity and customization
- Tested design system and screenshot comparisons on target display/scaling.
- Settings UI for fonts, colors, tab spacing, hotkeys and themes.
- Accessibility and contrast modes.
- Font/profile persistence and better tab lifecycle.
- Optional themes and keyboard command palette, scoped separately.

## v0.3 — Productivity
- Split panes and per-pane PTY management.
- SSH profiles using normal OpenSSH agent and config.
- Workspace layouts, restore *paths and profiles*, not imaginary live processes.
- Search, hyperlinks, terminal find, terminal title/profiles.

## Experimental / only after research
- Native GNOME Wayland blur capability (no hacks or screen scraping).
- Plugin architecture, developer panels, CTF project profiles.
- Fedora RPM/desktop packaging; releases, signing and update strategy.
- No AI-initiated shell execution without explicit human review.
