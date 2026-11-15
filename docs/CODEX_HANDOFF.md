# Codex handoff — Veyl v0.1

## Mission

Turn this **source foundation** into a reproducibly working, visually faithful native terminal on Fedora 45 GNOME Wayland. Do not substitute mock screenshots or treat a build as proof of runtime behavior.

Read [AGENTS.md](../AGENTS.md), [STATUS.md](STATUS.md), [Architecture](ARCHITECTURE.md), [Visual spec](VISUAL_SPEC.md), [Testing](TESTING.md), and [Security](SECURITY.md) before changing code.

## Highest-priority work items, in order

1. **Keep Fedora 45 checks green.** The appearance branch tracks Cargo.lock. Run the locked tests, lint, and build plus the formatting check documented in [Testing](TESTING.md).
2. **Run the remaining desktop checks.** [Status](STATUS.md) records the lifecycle audit and the native Bash, Vim, tmux, resize, clipboard action, and session cleanup results. Exercise the physical Ctrl+Shift shortcuts, keyboard navigation, and the remaining cases on Fedora GNOME Wayland.
3. **Finish visual verification.** Review native blur with Blur My Shell disabled, text contrast across backdrops, focus visibility, font fallback, and 100/125/150% scaling. Save screenshots only with permission.
4. **Review settings and session boundaries.** The independent security review and settings write hardening remain open. Review process behavior beyond the tested interactive Bash jobs before making stronger cleanup claims.
5. **Record results in Status.** Include the environment, commands, evidence, and remaining issues. Keep later enhancements in small PRs.

## v0.1 release gate

- GitHub CI green, dependencies installed with reproducible setup.
- No build warnings or clippy errors.
- Multiple tab lifecycle tests on GNOME Wayland (including closing last tab).
- No terminal text opacity regression.
- Shell/PTY compatibility smoke tests and standard clipboard shortcuts pass.
- No crash/hang/leak on 30 repetitive tab open/close cycles.
- Keyboard navigation, selection, font fallback and high-DPI behavior checked.
- README contains verified launch steps and known limitations.
- Independent security review of session disposal and settings permissions.

**Do not tag v0.1.0 as released until the above gates have evidence.**

## Prompt to start a Codex session

> Read AGENTS.md and the project docs. Check STATUS.md and the current PR for evidence, then work through the remaining Fedora 45 GNOME Wayland tests. Preserve the approved appearance, document the results, and keep post-v0.1 features outside the PR.
