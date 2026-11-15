# Codex instructions for Veyl

This file is the authoritative contributor entry point. Read this and the linked design/test docs **before editing code**.

## Product contract
- Independent **Fedora-first terminal emulator**. Visual target: the *look* of Windows Terminal on Windows 11, with dark translucency and restrained rounded corners. This does not mean implementing PowerShell, WSL, Microsoft APIs, or recreating Windows shell behavior.
- The user strongly prefers **translucent frosted glass** to plain transparency. Keep terminal glyphs fully opaque; a global window-opacity slider is the wrong implementation.
- Do not claim compositor blur works on GNOME Wayland without actual end-to-end testing. No Microsoft logo, trademark copy, or proprietary art.
- Every cosmetic setting should be user-customizable, not hardcoded as an immutable behavior. Sensible default values are fine.
- Do not turn Veyl into a themed shell script. Preserve VTE, PTY, clipboard, resize, and terminal accessibility fundamentals.

## Tech and sources
- Rust 2021, GTK4 (crate gtk4 v0.11), VTE4 (crate vte4 v0.10), serde/TOML.
- Target: Fedora 45, GNOME, Wayland. Keep Linux native; no Electron, browser engine, or cloud backend in v0.1.
- Primary source files: src/main.rs, src/window.rs, src/config.rs, assets/veyl.css.
- Reference: https://docs.rs/vte4/0.10.0/vte4/ and https://gtk-rs.org/gtk4-rs/.

## Required workflow
1. Inspect the current tree, issues and CI; do not overwrite unrelated work.
2. Reproduce current failures. Run `cargo fmt -- --check`, `cargo test`, `cargo clippy --all-targets -- -D warnings` and `cargo build` **on a system with GTK4/VTE4 development packages**.
3. If testing is unavailable, disclose it in the PR and STATUS.md; do not mark a feature DONE on assumptions.
4. Implement one scoped change with tests and user-facing documentation. Keep environment-specific blur behind a testable capability gate and degrade to an opaque/dark fallback.
5. Validate directly under GNOME Wayland, including with blur extensions disabled, before calling appearance reproducible. Collect non-sensitive screenshots only with permission.
6. Use feature branches and PRs for new work; no force pushes and no unreviewed `main` changes after bootstrap.
7. Never run untrusted shell commands, downloaded challenge samples, or privileged operations without explicit authorization.

## Definition of v0.1
- An actual native window opens; shell starts in a working PTY and supports typing, programs like `vim`, control sequences, resize and exit handling.
- New tab, select tab, close tab, copy and paste work.
- Windows Terminal-inspired header/tabs are present; opacity changes without dimming text, and persists across launches.
- Does not crash or leak terminal sessions under repeated open/close; errors are presented clearly.
- Documented setup steps are reproducible on Fedora 45. CI passes and a manual Wayland checklist is completed before marking v0.1 validated.
- All known limitations are explicitly documented in docs/STATUS.md.

## Non-goals until after validation
- True Windows Mica/Acrylic blur, split panes, command palette, SSH profiles, workspace restoration, AI, security tooling, plugins, cloud syncing, auto-update, RPM packaging, or pixel-perfect Windows frame controls.
- No automatic execution of suggested commands; no transcript logging, keystroke logging, telemetry, or silent network access.

## Key docs
- docs/CODEX_HANDOFF.md — next steps and acceptance gates.
- docs/ARCHITECTURE.md — PTY/session architecture and design boundaries.
- docs/VISUAL_SPEC.md — look, tokens and GNOME/Wayland limits.
- docs/TESTING.md — automated/manual QA.
- docs/SECURITY.md — threat model and lifecycle handling.
- docs/ROADMAP.md — scope over versions.
- docs/STATUS.md — verified vs not verified.
