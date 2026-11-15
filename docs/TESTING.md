# Test plan and quality gates

## Automated (required)

~~~bash
cargo fmt -- --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
~~~

Build prerequisites on Fedora 45:

~~~bash
sudo dnf install rust cargo gcc pkgconf-pkg-config gtk4-devel vte291-gtk4-devel
~~~

The GitHub Actions CI builds/tests within a Fedora 45 container. UI integration tests **cannot** be replaced by headless compilation.

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
| Opacity | Drag 0.40→0.86→1.00 | Only background fades; text stays crisp |
| Persistence | Set opacity, restart | Slider and appearance preserved |
| Config failure | Malformed TOML | Safe fallback plus clear error |
| Keyboard | Ctrl+Shift+T/W/C/V | Appropriate action |
| GNOME | Extensions off, Wayland session | App still usable |
| Scale | 100/125/150% | UI legible, tabs usable |
| Shell exit | `exit` | Tab closes; last tab closes app |
| Child failure | Set invalid `$SHELL`, start app | Fallback or actionable error; no hidden failure |

Do not use risky real malware or production credentials to test the terminal. Use synthetic output, ordinary TUI tools, and localhost tests.

## Documentation of results

Update STATUS.md with date, host and commands. Use:
- PASS — reproduced on the target platform
- FAIL — reproduced defect, include reproduction
- NOT RUN — no claimed verification

If an environment is unavailable, leave the status as NOT RUN rather than implying completion.
