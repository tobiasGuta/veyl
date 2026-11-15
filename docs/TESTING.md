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
