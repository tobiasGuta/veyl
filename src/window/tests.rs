use super::*;
use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

fn wait_for<T>(description: &str, mut check: impl FnMut() -> Option<T>) -> T {
    let context = glib::MainContext::default();
    let deadline = Instant::now() + Duration::from_secs(8);
    while Instant::now() < deadline {
        while context.pending() {
            context.iteration(false);
        }
        if let Some(result) = check() {
            return result;
        }
        thread::sleep(Duration::from_millis(5));
    }
    panic!("Timed out waiting for {description}");
}

fn terminal_text(terminal: &Terminal) -> String {
    let output = gio::MemoryOutputStream::new_resizable();
    terminal
        .write_contents_sync(&output, vte4::WriteFlags::Default, gio::Cancellable::NONE)
        .unwrap();
    output.close(gio::Cancellable::NONE).unwrap();
    String::from_utf8(output.steal_as_bytes().to_vec()).unwrap()
}

fn shell_process(terminal: &Terminal) -> PathBuf {
    wait_for("PTY attachment", || terminal.pty());
    terminal.feed_child(b"printf '\\nVEYL_TEST_PID=%s\\n' \"$$\"\r");
    wait_for("Bash PID from the PTY", || {
        terminal_text(terminal).lines().find_map(|line| {
            let pid = line.strip_prefix("VEYL_TEST_PID=")?.parse::<u32>().ok()?;
            Some(PathBuf::from(format!("/proc/{pid}")))
        })
    })
}

fn foreground_sleep(terminal: &Terminal, shell: &Path) -> PathBuf {
    terminal.feed_child(b"sleep 120\r");
    let pid = shell.file_name().unwrap();
    let children = shell.join("task").join(pid).join("children");
    wait_for("foreground sleep to start", || {
        let children = fs::read_to_string(&children).ok()?;
        let child = PathBuf::from("/proc").join(children.split_whitespace().next()?);
        (fs::read_to_string(child.join("comm")).ok()?.trim() == "sleep").then_some(child)
    })
}

fn check_resize_and_scrollback(window: &ApplicationWindow, terminal: &Terminal) {
    let pty = terminal.pty().unwrap();
    let initial = pty.size().unwrap();
    window.maximize();
    let (rows, columns) = wait_for("Wayland resize to reach the PTY", || {
        let size = pty.size().unwrap();
        (size != initial && window.is_maximized()).then_some(size)
    });
    terminal.feed_child(b"printf '\\nVEYL_TEST_SIZE='; stty size\r");
    wait_for("Bash to report the resized PTY dimensions", || {
        terminal_text(terminal)
            .contains(&format!("VEYL_TEST_SIZE={rows} {columns}"))
            .then_some(())
    });
    window.unmaximize();
    wait_for("PTY size to return to the unmaximized size", || {
        (pty.size().unwrap() == initial && !window.is_maximized()).then_some(())
    });

    terminal
        .feed_child(b"printf '\\033[2J\\033[H\\033[31mVEYL_ANSI\\033[0m\\033[4;7H'; read -r _\r");
    wait_for("VTE to render the ANSI text and move the cursor", || {
        (terminal_text(terminal).contains("VEYL_ANSI") && terminal.cursor_position().0 == 6)
            .then_some(())
    });
    terminal.feed_child(b"\r");
    terminal.feed_child(b"printf 'VEYL_LINE_%03d\\n' {1..160}\r");
    wait_for("both ends of the scrollback to be retained", || {
        let text = terminal_text(terminal);
        (text.contains("VEYL_LINE_001") && text.contains("VEYL_LINE_160")).then_some(())
    });
}

fn check_terminal_programs(terminal: &Terminal) {
    if glib::find_program_in_path("tmux").is_some() {
        terminal.feed_child(b"tmux -L veyl-qa-$$ -f /dev/null new-session 'echo VEYL_TMUX_RUNNING; read -r _'; echo VEYL_TMUX_EXITED\r");
        wait_for("tmux pane to start", || {
            terminal_text(terminal)
                .lines()
                .any(|line| line.trim() == "VEYL_TMUX_RUNNING")
                .then_some(())
        });
        terminal.feed_child(b"\r");
        wait_for("tmux to return to the outer Bash", || {
            terminal_text(terminal)
                .lines()
                .any(|line| line.trim() == "VEYL_TMUX_EXITED")
                .then_some(())
        });
        eprintln!("tmux pane and return to Bash passed");
    }

    if glib::find_program_in_path("vim").is_some() || glib::find_program_in_path("vi").is_some() {
        let fixture = PathBuf::from(std::env::var_os("HOME").unwrap()).join("veyl-qa-vim.txt");
        fs::write(&fixture, "alpha\nbeta\n").unwrap();
        terminal.feed_child(b"editor=$(command -v vim || command -v vi); \"$editor\" -Nu NONE -i NONE -n -T vt100 +2 ~/veyl-qa-vim.txt\r");
        wait_for("Vim to show the fixture", || {
            terminal_text(terminal)
                .contains("alpha\nbeta")
                .then_some(())
        });
        terminal.feed_child(b"\x1bOAiX\x1b:wq\r");
        wait_for("Vim to move up and edit the first line", || {
            (fs::read_to_string(&fixture).ok().as_deref() == Some("Xalpha\nbeta\n")).then_some(())
        });
        fs::remove_file(fixture).unwrap();
        eprintln!("Vim cursor motion and file edit passed");
    }
}

fn activate(window: &ApplicationWindow, action: &str) {
    gio::prelude::ActionGroupExt::activate_action(window, action, None);
}

fn test_application() -> Application {
    gtk::init().unwrap();
    assert!(gdk::Display::default().unwrap().is::<WaylandDisplay>());
    let app = Application::builder()
        .application_id("io.github.tobiasGuta.Veyl.Tests")
        .flags(gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.register(gio::Cancellable::NONE).unwrap();
    app
}

// Run with the isolated Bash HOME documented in docs/TESTING.md.
#[test]
#[ignore = "requires a Fedora GNOME Wayland desktop and an isolated Bash HOME"]
fn wayland_session_lifecycle() {
    let app = test_application();

    let (window, state) = create_window(&app, Config::default());
    let weak_window = window.downgrade();
    let weak_state = Rc::downgrade(&state);
    window.present();
    open_tab(&state);

    let first = active_terminal(&state).unwrap();
    let first_process = shell_process(&first);
    assert_eq!(
        fs::read_to_string(first_process.join("comm"))
            .unwrap()
            .trim(),
        "bash"
    );
    assert!(fs::read_link(first_process.join("fd/0"))
        .unwrap()
        .starts_with("/dev/pts"));
    assert_eq!(
        fs::read_link(first_process.join("cwd")).unwrap(),
        PathBuf::from(std::env::var_os("HOME").unwrap())
    );
    check_resize_and_scrollback(&window, &first);
    check_terminal_programs(&first);

    for _ in 0..30 {
        activate(&window, "new-tab");
        let terminal = active_terminal(&state).unwrap();
        assert_ne!(terminal, first);
        let process = shell_process(&terminal);
        let weak_terminal = terminal.downgrade();

        drop(terminal);
        activate(&window, "close-tab");
        assert_eq!(active_terminal(&state).as_ref(), Some(&first));
        wait_for("closed terminal and Bash to be released", || {
            (!process.exists() && weak_terminal.upgrade().is_none()).then_some(())
        });
    }

    let mut cancelled = Vec::new();
    for _ in 0..30 {
        activate(&window, "new-tab");
        cancelled.push(active_terminal(&state).unwrap().downgrade());
        activate(&window, "close-tab");
    }
    assert_eq!(state.borrow().tabs.len(), 1);
    wait_for("terminals closed during spawning to be released", || {
        cancelled
            .iter()
            .all(|terminal| terminal.upgrade().is_none())
            .then_some(())
    });

    activate(&window, "new-tab");
    let second = active_terminal(&state).unwrap();
    let second_process = shell_process(&second);
    let first_select = state.borrow().tabs[0].select.clone();
    first_select.emit_clicked();
    assert_eq!(active_terminal(&state).as_ref(), Some(&first));
    let sleep = foreground_sleep(&first, &first_process);
    drop(first);
    activate(&window, "close-tab");
    wait_for("closed tab's shell and foreground sleep to exit", || {
        (!first_process.exists() && !sleep.exists()).then_some(())
    });

    open_tab_with_shell(&state, "/dev/null", None);
    let failed = active_terminal(&state).unwrap();
    wait_for("spawn error text to show in the terminal", || {
        (!failed.is_input_enabled() && terminal_text(&failed).contains("could not start the shell"))
            .then_some(())
    });
    assert!(failed.pty().is_none());
    activate(&window, "close-tab");
    assert_eq!(active_terminal(&state).as_ref(), Some(&second));

    let sleep = foreground_sleep(&second, &second_process);
    activate(&window, "new-tab");
    let third = active_terminal(&state).unwrap();
    let third_process = shell_process(&third);
    let other_sleep = foreground_sleep(&third, &third_process);
    let weak_second = second.downgrade();
    let weak_third = third.downgrade();
    drop((second, third));
    drop(state);
    window.close();
    drop(window);
    wait_for(
        "window, state, shells, and their foreground processes to exit",
        || {
            (weak_window.upgrade().is_none()
                && weak_state.upgrade().is_none()
                && weak_second.upgrade().is_none()
                && weak_third.upgrade().is_none()
                && !second_process.exists()
                && !third_process.exists()
                && !sleep.exists()
                && !other_sleep.exists())
            .then_some(())
        },
    );

    let (window, state) = create_window(&app, Config::default());
    window.present();
    open_tab(&state);
    let terminal = active_terminal(&state).unwrap();
    shell_process(&terminal);
    terminal.feed_child(b"exit\r");
    wait_for("last shell exit to close the window", || {
        (app.windows().is_empty() && state.borrow().tabs.is_empty()).then_some(())
    });

    let (window, state) = create_window(&app, Config::default());
    window.present();
    open_tab(&state);
    activate(&window, "close-tab");
    assert!(app.windows().is_empty());
}

#[test]
#[ignore = "requires an isolated Wayland session because it replaces the clipboard"]
fn wayland_clipboard_actions() {
    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    window.present();
    open_tab(&state);
    let terminal = active_terminal(&state).unwrap();
    shell_process(&terminal);

    for (action, expected) in [
        ("win.new-tab", "<Primary><Shift>t"),
        ("win.close-tab", "<Primary><Shift>w"),
        ("win.copy", "<Primary><Shift>c"),
        ("win.paste", "<Primary><Shift>v"),
    ] {
        let accelerators = app.accels_for_action(action);
        assert_eq!(accelerators.len(), 1);
        assert_eq!(
            gtk::accelerator_parse(&accelerators[0]),
            gtk::accelerator_parse(expected)
        );
    }

    let clipboard = window.clipboard();
    terminal.feed_child(b"printf '\\n\\033[38;2;202;17;43mVEYL_COPY_SELECTION\\033[0m\\n'\r");
    wait_for("copy fixture to appear in the terminal", || {
        terminal_text(&terminal)
            .lines()
            .any(|line| line == "VEYL_COPY_SELECTION")
            .then_some(())
    });
    terminal.select_all();
    activate(&window, "copy");
    let copied = Rc::new(RefCell::new(None));
    clipboard.read_text_async(gio::Cancellable::NONE, {
        let copied = copied.clone();
        move |result| {
            copied.replace(Some(result));
        }
    });
    let content = wait_for("clipboard text to be read", || copied.borrow_mut().take());
    assert!(content
        .unwrap()
        .unwrap()
        .contains("\nVEYL_COPY_SELECTION\n"));

    terminal.copy_clipboard_format(vte4::Format::Html);
    let html = Rc::new(RefCell::new(None));
    glib::MainContext::default().spawn_local({
        let html = html.clone();
        let clipboard = clipboard.clone();
        async move {
            let (stream, _) = clipboard
                .read_future(&["text/html;charset=utf-8"], glib::Priority::DEFAULT)
                .await
                .unwrap();
            let mut output = Vec::new();
            loop {
                let bytes = stream
                    .read_bytes_future(4096, glib::Priority::DEFAULT)
                    .await
                    .unwrap();
                if bytes.is_empty() {
                    break;
                }
                output.extend_from_slice(&bytes);
            }
            html.replace(Some(String::from_utf8(output).unwrap()));
        }
    });
    let html = wait_for("VTE to export the rendered ANSI color", || {
        html.borrow_mut().take()
    });
    assert!(html.contains("<font color=\"#CA112B\">VEYL_COPY_SELECTION</font>"));

    terminal.unselect_all();
    terminal.feed_child(
        b"printf '\\nVEYL_READY\\n'; read -r value; printf '\\nVEYL_PASTE=%s\\n' \"$value\"\r",
    );
    wait_for("the shell to wait for pasted input", || {
        terminal_text(&terminal)
            .lines()
            .any(|line| line == "VEYL_READY")
            .then_some(())
    });
    clipboard.set_text("veyl-paste-test");
    activate(&window, "paste");
    wait_for("pasted text to reach the PTY", || {
        terminal_text(&terminal)
            .lines()
            .any(|line| line == "veyl-paste-test")
            .then_some(())
    });
    terminal.feed_child(b"\r");
    wait_for("Bash to read the pasted value", || {
        terminal_text(&terminal)
            .lines()
            .any(|line| line == "VEYL_PASTE=veyl-paste-test")
            .then_some(())
    });
    drop(terminal);
    window.close();
}
