# Verification status

Updated: 2026-11-15 (initial v0.1 source bootstrap and Fedora CI verification)

**Classification:** v0.1 development foundation, **not yet validated as a daily-driver desktop application**. The Rust project **does compile in Fedora 45 CI**. A successful GUI launch and real GNOME Wayland visual check have **not** been demonstrated yet.

## Verified results

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
| Cargo unit tests | PASS | 3 tests in Actions #43757202645 |
| Cargo fmt / clippy | NOT RUN | Required as next CI improvement |
| Cargo.lock tracked | NOT DONE | Create during Codex hardening |
| Terminal PTY/shell launch | NOT RUN | Native desktop test required |
| Tab management | NOT RUN | Native test and signal lifecycle audit |
| Clipboard shortcuts | NOT RUN | GUI test required |
| GTK background opacity | NOT RUN | Actual compositor rendering test required |
| True compositor background blur | NOT IMPLEMENTED | No Acrylic/Mica claim |
| Fedora 45 GNOME Wayland manual testing | NOT RUN | Access to an actual target desktop required |
| Security review | NOT RUN | Child cleanup, signal reference cycles and settings permissions need audit |

## Outstanding v0.1 release gates

Read [the Codex handoff](CODEX_HANDOFF.md) and [testing checklist](TESTING.md); track this work in [GitHub issue #1](https://github.com/tobiasGuta/veyl/issues/1). **Do not tag a release** until the desktop, lifecycle, formatting and security checks have evidence.

Do not change PASS/NOT RUN entries based on inference. Record the exact command/environment, link to the run or PR, and keep unverified claims separate from verified ones.
