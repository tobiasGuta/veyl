# Verification status

## Rendered appearance and settings-path hardening

Fedora 45, GNOME Shell and Mutter 51.1, GTK 4.24.1, VTE 0.84.1. An isolated 1920×1080 GNOME Wayland compositor ran with Blur My Shell absent. Tab overflow, split pane geometry, and Settings appearance tests passed at 100%, 125%, and 150%. Wayland traces confirmed the requested fractional scale and a native blur region on each run.

Test-only captures used opaque black, white, and a hard black–white edge as backdrops. At 72% opacity, the edge spanned roughly 30, 38, and 45 physical pixels at the three scales (10%–90%, sampled on a row inside the terminal). With plain translucency at 100% display scale the edge changed in a single pixel step. The opaque 100%-opacity window had the same background over white and black. The solid default-foreground sample stayed at RGB 232, 234, 240 across all captures.

| Default foreground at 100% display scale | Black backdrop | White backdrop |
| --- | --- | --- |
| Frosted at 72% opacity | 16.13:1 | 6.27:1 |
| Plain translucency at 72% opacity | 16.13:1 | 6.27:1 |
| Plain translucency at 40% opacity | 16.75:1 | 2.17:1 |
| Solid background at 100% opacity | 15.54:1 | 15.54:1 |

The default setting clears the 4.5:1 reference for the measured samples. Low opacity over bright backgrounds reduces readability. These measurements cover the default foreground, the tested backdrops and scales, and the fixture’s window and text. [Contrast reference](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html)

Settings-path tests verified that Veyl refuses non-regular files, files writable by other users, and unsafe parent directories. A safe dotfiles symlink still points to the original file after saving. The appearance, Bash prompt, smart prompt, profile editor, configuration recovery, and session lifecycle regressions also passed. The independent session and settings security review is still pending.

## Physical shortcuts and split panes

The user reported that the physical shortcuts they tried passed except for Alt+Shift+Minus. The follow-up registers the shifted Minus key symbol for Split down and translates an existing legacy Split down binding. Fedora GNOME Wayland automated tests checked both split orientations, divider and padding geometry, and the effective shortcut. The user retested the installed build and reported that the physical keyboard shortcuts passed, including the revised Alt+Shift+Minus. Visual comparison of the revised pane styling remains open.

## Tab overflow

The GNOME Wayland test opened enough tabs to overflow the titlebar, verified the horizontal scrollbar was hidden, and checked that cycling to the first tab and opening Settings brought those tabs into view.

## Fedora preview candidate

The candidate at `707c0de` passed formatting, locked unit tests, Clippy, and the optimized build on Fedora 45. The isolated GNOME Wayland suite passed 14 of 17 display cases, including clipboard actions, Vim, tmux, and repeated PTY lifecycle coverage. Search focus, directional split focus, and confirmed multiline paste each failed in that headless session. Each passed on three reruns on the signed-in GNOME desktop with fresh test settings. The appearance and window control tests also passed with Blur My Shell disabled, and the Wayland trace showed Mutter receiving the native blur region.

The signed-in runs exercised GTK actions, and the isolated run used a separate clipboard. The physical shortcut check passed in the follow-up above. The rendered appearance checks are recorded above. Pointer gestures, Orca, and a fresh-account app-grid check still need hands-on verification. The settings and session boundary review in this pass was performed by the implementation author. The independent review required by the release checklist remains open. The preview tag remains pending. See [draft release notes](releases/v0.1.0-preview.md).

## Pane zoom and directory titles

The Fedora 45 GNOME Wayland tests exercised zoom and restore in nested splits, separate font scales, closing a zoomed pane, and OSC 7 directory labels with manual rename precedence. Physical shortcuts and optical review remain open.

## Smart Bash prompt

The Fedora 45 GNOME Wayland test confirmed that the editor honors a disabled segment and that new Bash 5.3 tabs show the saved prompt, command failure status, and elapsed time without a separate .bashrc helper. The existing prompt and managed .bashrc regression passed. Physical UI review and the complete matrix of prompt indicators remain open.

## Profile editor

The Fedora 45 GNOME Wayland test confirmed profile creation, duplication, renaming, default selection across rename, preservation of unknown profile fields and comments, and launch from the updated menu. Mouse and keyboard navigation remain for manual review.

## Terminal links

The Fedora 45 GNOME Wayland test detected plain and OSC web links, rejected an OSC file link, and activated Find through the native context action group. Physical pointer gestures and launches into external applications remain open for manual review.

## Multiline paste

The Fedora 45 GNOME Wayland test confirmed that canceling keeps synthetic multiline text out of Bash and confirmation delivers it. The preview applies to Veyl's Paste action. Physical clipboard gestures and foreground-job close confirmation remain open.

## Settings recovery

The Fedora 45 GNOME Wayland test launched with malformed TOML, found the error and Open config file action, and confirmed defaults and preservation of the original TOML. Launching an external text editor remains for manual verification.

## Local installer

On Fedora 45, the installer created an executable, desktop entry and SVG in isolated home and data directories. The desktop entry passed validation and `gtk-launch` started Veyl with a home path containing spaces. The optimized build also started via the installed desktop entry and presented a Wayland window buffer in an isolated D-Bus session. The installed icon decoded at 48 px. Uninstall removed the three installed files and preserved settings. Visual app-grid testing on a fresh desktop account remains open.

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

**Classification:** v0.1 development foundation. This branch passed the native lifecycle regression tests. See the follow-up above for the physical shortcut pass. Visual checks remain open.

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
| Physical keyboard shortcuts | PASS | User tested the installed split-pane follow-up on their Fedora GNOME desktop and confirmed the shortcuts passed |
| Wayland blur and opacity controls | PROTOCOL PASS | With Blur My Shell disabled, GTK sent a rounded blur region to Mutter. The region cleared at 100%, returned at 72%, and cleared when frosted mode was turned off. The slider also accepted 40% |
| Fallback and persistence | PASS | XWayland reported blur unavailable and allowed tint adjustment after frosted mode was disabled. A Wayland relaunch restored 55% and frosted mode off |
| Visual blur, text contrast, scaling | PASS WITH LIMITATION | Verified rendered native blur and the default foreground over synthetic backdrops at 100%, 125%, and 150%. At 40% opacity, bright backgrounds can make text hard to read |
| Security review | PARTIAL | Settings-path hardening and PTY lifecycle regressions passed. Independent review is pending |

Run the two display tests using [the testing guide](TESTING.md#wayland-regression-tests). Native runs used `G_DEBUG=fatal-criticals`. A separate appearance smoke run still emitted GDK's duplicate frame-timing warning during Wayland popup presentation. The controls continued to work.

The GNOME capability check is sufficient for the tested Mutter 51.1 setup. Its protocol advertises blur at bind time. The registry query cannot detect a compositor that leaves the protocol manager present and withdraws the blur capability at runtime. Veyl currently evaluates availability when creating the window.

VTE 0.84.1 crashed during validation when explicitly unsetting a live PTY through the nullable setter. Veyl uses terminal widget disposal for normal teardown. Detached jobs and commands that ignore hangup can outlive their tab.

The [bootstrap Fedora 45 CI run](https://github.com/tobiasGuta/veyl/actions/runs/43757202645) covered three configuration tests and a debug build. [GitHub issue #1](https://github.com/tobiasGuta/veyl/issues/1) tracks the remaining v0.1 release gates.
