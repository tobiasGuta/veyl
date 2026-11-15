# Codex handoff — Veyl v0.1

## Mission

Turn this **source foundation** into a reproducibly working, visually faithful native terminal on Fedora 45 GNOME Wayland. Do not substitute mock screenshots or treat a build as proof of runtime behavior.

Read [AGENTS.md](../AGENTS.md), [STATUS.md](STATUS.md), [Architecture](ARCHITECTURE.md), [Visual spec](VISUAL_SPEC.md), [Testing](TESTING.md), and [Security](SECURITY.md) before changing code.

## Highest-priority work items, in order

1. **Compile and establish clean CI.** Use Fedora 45 or a Fedora container with `rust cargo gcc pkgconf-pkg-config gtk4-devel vte291-gtk4-devel`. Run `cargo build`, tests, fmt, and clippy; diagnose actual binding compatibility rather than guessing. Commit Cargo.lock for this application and use `--locked` in CI after generating it.
2. **Fix GTK ownership/lifecycle concerns.** Audit all `Rc<RefCell<_>>` callbacks. Prefer `Weak` references for signal handlers where capturing State can create widget → signal → State → widget cycles. Ensure Ctrl+Shift+W does not hold a RefCell borrow during closure. Test close while process runs, child-exited while closing, and tab close with concurrent GTK events.
3. **Native Fedora 45 runtime QA.** Open a Wayland window. Run `echo hello`, `printf '\033[31mRED\033[0m\n'`, `ls --color`, `vim`/`nvim`, `htop`, `ssh` and `tmux` (as available), resize the window, and check shell exit. Ensure errors surface inside the UI and not just stderr.
4. **Appearance QA.** Check opacity slider truly affects only background and is persisted. Test with no GNOME blur extensions. Ensure 40% is still readable, no white flashes, controls accessible, and tabs don't overflow. Save screenshots only if user authorizes them.
5. **Improve reliable user-facing behavior.** Ensure close tab behaves predictably and warns about running jobs when warranted; no silently orphaned jobs. Optionally support a standard desktop entry only after native smoke test.
6. **Reflect truth in docs.** Update STATUS.md with exact environment, commands, results, timestamps and unresolved issues. Split later enhancements into small PRs and report tested/untested separately.

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

> Inspect AGENTS.md and all docs, then compile the Veyl v0.1 source on Fedora 45. Work in a feature branch. First resolve build errors and tab/session lifecycle bugs, add a Cargo.lock and green CI, and record exact verification in docs/STATUS.md. Preserve the Windows Terminal-inspired dark translucent appearance, do not fake GNOME background blur, and do not add post-v0.1 features. Open a PR with evidence and remaining risks.
