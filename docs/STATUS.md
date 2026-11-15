# Verification status

Updated: 2026-11-15 (tab strip, frosted background, and desktop smoke testing)

**Classification:** v0.1 development foundation. The app launches on Fedora 45 GNOME Wayland. The full desktop release checklist is still open.

## Appearance branch verification

- Fedora 45 Workstation with GNOME Shell/Mutter 51.1, GTK 4.24.1 and VTE 0.84.1. Wayland at 1920×1080 and 100% scale. Fontconfig resolved Cascadia Mono to Cascadia Mono NF.
- `cargo fmt -- --check`, `cargo test --locked` (4 passed), `cargo clippy --locked --all-targets -- -D warnings`, and `cargo build --locked` passed locally.
- With Blur My Shell temporarily disabled, `GDK_BACKEND=wayland WAYLAND_DEBUG=1` showed GTK sending a rounded `set_blur_region` to Mutter. The opacity slider was exercised at 40%, 72%, and 100%. The protocol trace showed the region being cleared at 100%, restored at 72%, and cleared when frosted mode was toggled off.
- GTK accessibility actions opened the popover and changed both settings. Relaunching restored 55% opacity and frosted mode off. The tab smoke test opened seven tabs, verified the newest tab scrolled into view, and confirmed the previous tab became selected when it closed.
- A test shell (`/bin/sh`) ran on a `/dev/pts/*` PTY. On `GDK_BACKEND=x11` under XWayland, the popover reported the unsupported blur path and enabled the opacity slider after frosted mode was turned off.
- A Wayland run emitted a GDK duplicate frame-timing warning during popup presentation. The appearance and tab controls continued to work.
- Pixel-level blur fidelity, legibility across backdrops, and fractional scaling still need visual review. Low opacity over bright content can reduce contrast.

## Bootstrap CI results

- **GitHub main source commit:** [d51388d](https://github.com/tobiasGuta/veyl/commit/d51388dbced6a8b841148b1ad09a4e6d3d9986b5)
- **Green Fedora 45 build:** [GitHub Actions run #43757202645](https://github.com/tobiasGuta/veyl/actions/runs/43757202645)
- **Native dependencies in CI:** GTK 4.24.1, VTE 0.84.1 (reported by pkg-config).
- **Rust tests:** `cargo test` — 3 passed, 0 failed (config normalization and compatibility).
- **Rust build:** `cargo build` — succeeded.
- Fixed the GTK4 `v4_10` feature mismatch and a Rust ownership error discovered by earlier CI runs.
- Fixed an additional runtime RefCell borrow issue in the tab-closing action, but **runtime behavior remains untested**.

| Requirement | Status | Evidence / gap |
| --- | --- | --- |
| Repository initialization and docs | PASS | Source, AGENTS.md and docs on main |
| Fedora 45 Rust build | PASS | Actions #43757202645 |
| Cargo unit tests | PASS | 4 tests locally on the appearance branch |
| Cargo fmt / clippy | PASS | Local checks on Fedora 45 |
| Cargo.lock | TRACKED | CI uses `--locked` |
| Terminal PTY/shell launch | SMOKE PASS | `/bin/sh` attached to a PTY on GNOME Wayland |
| Tab management | SMOKE PASS | Selection, overflow and closure on GNOME Wayland. Lifecycle stress test and audit pending |
| Clipboard shortcuts | NOT RUN | GUI test required |
| GTK background opacity | SMOKE PASS | Slider and settings write exercised. Visual contrast review pending |
| Compositor background blur | PROTOCOL PASS | Mutter received and cleared blur regions with Blur My Shell disabled. Visual comparison pending |
| Fedora 45 GNOME Wayland manual testing | PARTIAL | Native launch and accessibility-driven smoke tests. See remaining checklist |
| Security review | NOT RUN | Child cleanup, signal reference cycles and settings permissions need audit |

## Outstanding v0.1 release gates

Read [the Codex handoff](CODEX_HANDOFF.md) and [testing checklist](TESTING.md); track this work in [GitHub issue #1](https://github.com/tobiasGuta/veyl/issues/1). **Do not tag a release** until the desktop, lifecycle, formatting and security checks have evidence.

Do not change PASS/NOT RUN entries based on inference. Record the exact command/environment, link to the run or PR, and keep unverified claims separate from verified ones.
