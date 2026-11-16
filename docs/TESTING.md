# Test plan and quality gates

## Automated (required)

~~~bash
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
~~~

Build prerequisites on Fedora 45:

~~~bash
sudo dnf install rust cargo rustfmt clippy gcc pkgconf-pkg-config gtk4-devel vte291-gtk4-devel
~~~

The GitHub Actions CI builds/tests within a Fedora 45 container. UI integration tests **cannot** be replaced by headless compilation.

## Wayland regression tests

Run from the repository on Fedora GNOME Wayland. An isolated Bash home keeps the test commands, Vim fixture, and Veyl settings apart from your own. The test exercises 30 live tab cycles, 30 closes during launch, resize, scrollback, shell launch failure, and foreground job cleanup. It runs Vim and tmux when available.

~~~bash
qa_dir=$(mktemp -d)
mkdir "$qa_dir/home"
touch "$qa_dir/home/.bashrc"
env -u BASH_ENV -u ENV CARGO_HOME="${CARGO_HOME:-$HOME/.cargo}" \
  HOME="$qa_dir/home" XDG_CONFIG_HOME="$qa_dir/config" SHELL=/bin/bash \
  GDK_BACKEND=wayland GTK_A11Y=none G_DEBUG=fatal-criticals \
  cargo test --locked wayland_session_lifecycle -- --ignored --nocapture
~~~

Run the same command with `wayland_clipboard_actions` in an isolated Wayland session. That test replaces the session clipboard with synthetic text, exercises the application's copy and paste actions, and checks the ANSI foreground color in VTE's HTML export. Verify the physical shortcuts separately with the checklist below.

Run the command with `wayland_appearance_settings` and a fresh `XDG_CONFIG_HOME` to test the Settings tab, font preview, switching between terminal and Settings tabs, scheme and default-profile selection, blur and opacity persistence, Discard changes, Save, Reset to defaults, and TOML comment preservation.

Run the same command with `wayland_tab_interactions` to test ordered tab cycling, menu actions on a background tab, rename and reset, tab color rules, and shell cleanup when a tab closes with its menu open. For manual coverage, right-click an inactive tab, use Shift+F10 on a focused tab, choose a light and a dark tab color, then press Ctrl+Tab and Ctrl+Shift+Tab at the ends of the strip.

The same harness also runs these tests. Use a fresh test configuration directory for each run.

- `wayland_search_and_tab_organization` covers literal search in long scrollback, two independent search fields, focus restoration, same-folder duplication, and consistency between model and widget tab order.
- `wayland_profiles_splits_and_progress` covers a named project profile, rendered ANSI colors, progress updates and reset, nested splits, directional focus, and shell cleanup after pane and window close.
- `wayland_config_recovery` uses fresh settings to check the startup error, the config path and Open config file button, and the fallback settings.
- `wayland_multiline_paste_preview` uses synthetic text and checks that Cancel keeps it out of Bash and Paste delivers it.
- `wayland_command_palette` checks search, empty results, custom shortcut labels, and action and profile activation.
- `wayland_shortcut_editor` checks conflict feedback, persistence, and application of a valid shortcut on Save.

- `wayland_bash_prompt` requires an empty isolated Bash home. It exercises local prompt persistence, a theme that sets PS1 from a hook, the original Bash aliases, and the cancel, apply, and remove actions for the isolated `.bashrc`.

Manually verify pointer drag and drop, physical accelerator dispatch, and divider sizing on your display. In Settings, check sidebar navigation, menu row visibility, and the fixed footer at narrow window sizes and fractional scaling.

Run `wayland_window_controls` with the same harness to check the compact tab control dimensions, new-tab action, tab menu, three window control widths, native maximize and restore, and window close. Compare idle, hover, and maximized glyphs visually at the target display scales.

## Manual GNOME Wayland checklist

Record Fedora version, VTE/GTK versions, GNOME version, Wayland/X11 session, resolution, display scaling, font availability, and whether Blur My Shell is enabled.

| Case | Test | Expected |
| --- | --- | --- |
| Launch | `cargo run` | Native window shown; no crash |
| PTY | `tty` | `/dev/pts/*` |
| Shell | `echo Veyl`; `pwd` | Shell works, starts in user home |
| ANSI | `printf '\033[32mok\033[0m\n'` | Green “ok” then normal |
| Cursor | Open `vim` or `nvim` | Arrow navigation and redraw |
| Resize | Resize window repeatedly | App and TUI reflow correctly |
| Scrollback | Output >100 lines | History scrolls without locking up |
| Tabs | New, switch, close tabs repeatedly | Isolated sessions, right tab focused |
| Multiple jobs | Run `sleep 120` in one tab and close it | Child/process behavior documented |
| Clipboard | Select text, Ctrl+Shift+C/V | Copy/paste as expected |
| Opacity | Drag 40%→72%→100% | Only background fades; text stays crisp |
| Frosted background | Toggle on and off over a high-contrast backdrop with Blur My Shell disabled | Wayland blur smooths the backdrop in frosted mode; plain translucency shows its sharp edges |
| Blur unavailable | Launch with `GDK_BACKEND=x11` if XWayland is available | Default background is solid, and turning off frosted mode enables the opacity slider |
| Persistence | Change opacity and toggle frosted background, then restart | Both settings are preserved |
| Config failure | Malformed TOML | Safe fallback plus clear error |
| Keyboard | Ctrl+Shift+T/W/C/V | Appropriate action |
| GNOME | Extensions off, Wayland session | App still usable |
| Scale | 100/125/150% | UI legible, tabs usable |
| Shell exit | `exit` | Tab closes; last tab closes app |
| Child failure | Point `$SHELL` at a non-executable regular file and start the app | The tab shows an actionable error, blocks input, and closes normally |
| Close during launch | Immediately close a new tab, then repeat on the last tab | Launch cancels cleanly and no late callback affects another tab |
| Window shutdown | Start `sleep 120` in two Bash tabs, then close the window | Both tabs release their terminals and the two foreground jobs exit |

Do not use risky real malware or production credentials to test the terminal. Use synthetic output, ordinary TUI tools, and localhost tests.

## Documentation of results

Update STATUS.md with date, host and commands. Use:
- PASS — reproduced on the target platform
- FAIL — reproduced defect, include reproduction
- NOT RUN — no claimed verification

If an environment is unavailable, leave the status as NOT RUN rather than implying completion.
