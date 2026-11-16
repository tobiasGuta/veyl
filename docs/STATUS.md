# Verification status

## Profile editor

The Fedora 45 GNOME Wayland test confirmed profile creation, duplication, renaming, default selection across rename, preservation of unknown profile fields and comments, and launch from the updated menu. Mouse and keyboard navigation remain for manual review.

## Terminal links

The Fedora 45 GNOME Wayland test detected plain and OSC web links, rejected an OSC file link, and activated Find through the native context action group. Physical pointer gestures and launches into external applications remain open for manual review.

## Multiline paste

The Fedora 45 GNOME Wayland test confirmed that canceling keeps synthetic multiline text out of Bash and confirmation delivers it. The preview applies to Veyl's Paste action. Physical clipboard gestures and foreground-job close confirmation remain open.

## Settings recovery

The Fedora 45 GNOME Wayland test launched with malformed TOML, found the error and Open config file action, and confirmed defaults and preservation of the original TOML. Launching an external text editor remains for manual verification.

## Local installer

On Fedora 45, the installer created an executable, desktop entry and SVG in isolated home and data directories. The desktop entry passed validation and `gtk-launch` started Veyl with a home path containing spaces. Uninstall removed the three installed files. Visual app-grid testing on a fresh desktop account remains open.

## Command palette

On Fedora 45 GNOME Wayland, the command palette test exercised search, empty results, current shortcut labels, Settings activation, and opening a profile. Physical keyboard navigation remains for manual verification.

## Launcher icon

2026-11-16, Fedora 45 GNOME Wayland. `desktop-file-validate` accepted both the repository and installed desktop entries. GdkPixbuf rendered the SVG, and GTK resolved it by application ID through the installed icon theme at 16, 24, 48, and 128 px. Visual verification in the GNOME dock remains open.

## Bash prompt editor

On Fedora 45 GNOME Wayland with Blur My Shell disabled, the Bash prompt integration test confirmed that existing theme hooks and aliases still run, new Veyl Bash tabs use the saved prompt, and Save leaves the isolated `.bashrc` intact. The test also covered canceling the dotfile action, confirming the write, using the stored dotfile with Veyl's shell default, and confirming removal of the managed block. The appearance, shortcut, and terminal lifecycle tests passed. Physical input and visual review remain open.

## Settings tab redesign

On Fedora 45 GNOME Wayland with Blur My Shell disabled, the Settings, shortcut editor, tab menu, tab interaction, and profile/split integration tests passed with fatal GTK criticals enabled. The Settings test checks switching away and back, discarding and saving drafts, and keeping the workspace open when the last shell tab closes. Visual comparison and physical keyboard and pointer testing remain open.

## Window controls follow-up

On Fedora 45 GNOME Wayland, the `wayland_window_controls` test verified the 28 CSS px control widths and native maximize, restore, and close actions with fatal GTK warnings enabled. GTK still sent a native blur region with Blur My Shell temporarily disabled. The test also verified the 40 × 19 CSS px tab controls and tab menu. Visual comparison at fractional scales remains open.

## Terminal productivity follow-up

Fedora 45 GNOME Wayland regression tests covered literal and case-sensitive scrollback search, per-pane queries, same-folder duplication, model and widget tab order, shortcut conflicts and persistence, named profile launch, rendered ANSI colors, progress states, nested splits, directional focus, and cleanup on pane and window close. A separate headless GNOME Wayland session verified that copy and paste target the focused split and leave the other shell and its output intact. The tab, appearance, and lifecycle regressions also ran on this branch. The Wayland trace showed Mutter receiving a native blur region with Blur My Shell disabled. Physical drag and drop, keyboard dispatch, and visual contrast remain for desktop verification.

## Tab interactions follow-up

On Fedora 45 GNOME Wayland, `cargo test --locked wayland_tab_interactions -- --ignored --nocapture` passed with `G_DEBUG=fatal-criticals` and Blur My Shell disabled. The test exercised menu mapping, renaming and resetting a background tab, color styling and reset, forward and backward wraparound, and closing a tab with its menu open. The existing shell stayed alive through renaming and terminated on close. The Wayland trace still showed GTK submitting a native blur region. Physical pointer and keyboard gestures, color-picker interaction, and visual contrast remain for manual review.

## Appearance Settings follow-up

On Fedora 45 GNOME Wayland, `cargo test --locked wayland_appearance_settings -- --ignored --nocapture` passed with a fresh config directory and `G_DEBUG=fatal-criticals`. The test exercised the Settings entry, live font, cursor, and padding preview, Cancel, Save, and Reset to defaults. The original Bash process remained active, and TOML comments and unrelated keys survived saving. Native picker interaction and visual preview fidelity still need manual review.

Updated 2026-11-15 for [draft PR #2](https://github.com/tobiasGuta/veyl/pull/2).

**Classification:** v0.1 development foundation. This branch passed the native lifecycle regression tests. The physical keyboard shortcuts and visual checks remain open.

## Fedora 45 validation

Fedora 45 Workstation, GNOME Shell and Mutter 51.1, GTK 4.24.1, VTE 0.84.1. The desktop ran Wayland at 1920×1080 and 100% scale. Fontconfig resolved Cascadia Mono to Cascadia Mono NF. The clipboard test ran in a separate headless GNOME Wayland session with a private clipboard.

| Requirement | Result | Evidence |
| --- | --- | --- |
| Automated quality checks | PASS | `cargo fmt -- --check`, `cargo test --locked` (4 passed, 2 display tests ignored by default), `cargo clippy --locked --all-targets -- -D warnings`, and `cargo build --locked` |
| Bash and PTY | PASS | Bash started in the isolated home, attached to `/dev/pts/*`, and returned output over the PTY |
| Terminal resize | PASS | Maximize and restore updated the PTY dimensions. Bash's `stty size` agreed |
| ANSI color, cursor, and scrollback | PASS | VTE exported the requested RGB color, moved the cursor to the requested column, and retained the first and last of 160 output lines |
| Vim and tmux | PASS | Fedora's Vim 9.2 (`vi`, using vt100 terminfo) accepted an Up arrow and saved an edit to the first line. tmux 3.8 opened a pane and returned to Bash |
| Tab lifecycle | PASS | 30 running Bash tabs closed and released their terminal widgets and child processes. Another 30 immediate open/close cycles released the terminal widgets. Selection and overflow were exercised |
| Child exit and window shutdown | PASS | Last-shell exit closed the window. Closing a tab with `sleep 120` and closing a window with two foreground `sleep` jobs released the terminal widgets and the tested shell and job processes |
| Spawn failure | PASS | Launching the binary with a non-executable regular file in `$SHELL` showed actionable text in the tab. Closing the last tab returned process status 0. The regression test also checked that a failed launch left no PTY |
| Clipboard and shortcut bindings | PASS | On isolated headless GNOME Wayland, the app's copy and paste actions exchanged synthetic text and Bash read the pasted value. GTK reported all four expected accelerator bindings |
| Physical keyboard shortcuts | NOT RUN | The isolated compositor did not deliver the synthetic key events, so the actions were tested directly |
| Wayland blur and opacity controls | PROTOCOL PASS | With Blur My Shell disabled, GTK sent a rounded blur region to Mutter. The region cleared at 100%, returned at 72%, and cleared when frosted mode was turned off. The slider also accepted 40% |
| Fallback and persistence | PASS | XWayland reported blur unavailable and allowed tint adjustment after frosted mode was disabled. A Wayland relaunch restored 55% and frosted mode off |
| Visual blur, text contrast, scaling | NOT RUN | Pixel-level appearance, readability across backdrops, and fractional scaling still need visual review |
| Security review | PARTIAL | Session ownership and normal child disposal were reviewed. Independent review and settings-path hardening remain open |

Run the two display tests using [the testing guide](TESTING.md#wayland-regression-tests). Native runs used `G_DEBUG=fatal-criticals`. A separate appearance smoke run still emitted GDK's duplicate frame-timing warning during Wayland popup presentation. The controls continued to work.

The GNOME capability check is sufficient for the tested Mutter 51.1 setup. Its protocol advertises blur at bind time. The registry query cannot detect a compositor that leaves the protocol manager present and withdraws the blur capability at runtime. Veyl currently evaluates availability when creating the window.

VTE 0.84.1 crashed during validation when explicitly unsetting a live PTY through the nullable setter. Veyl uses terminal widget disposal for normal teardown. Detached jobs and commands that ignore hangup can outlive their tab.

The [bootstrap Fedora 45 CI run](https://github.com/tobiasGuta/veyl/actions/runs/43757202645) covered three configuration tests and a debug build. [GitHub issue #1](https://github.com/tobiasGuta/veyl/issues/1) tracks the remaining v0.1 release gates.
