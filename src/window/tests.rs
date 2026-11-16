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
        // Clear any terminal-query replies left on the Bash input line by tmux.
        terminal.feed_child(b"\x15editor=$(command -v vim || command -v vi); \"$editor\" -Nu NONE -i NONE -n -T vt100 +2 ~/veyl-qa-vim.txt\r");
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
    let other = active_terminal(&state).unwrap();
    let other_shell = shell_process(&other);
    activate(&window, "split-right");
    let terminal = active_terminal(&state).unwrap();
    assert_ne!(other_shell, shell_process(&terminal));

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
    assert!(!terminal_text(&other).contains("veyl-paste-test"));
    assert!(other_shell.exists());
    drop((terminal, other));
    window.close();
}

fn descendants(root: &gtk::Widget) -> Vec<gtk::Widget> {
    let mut widgets = vec![root.clone()];
    let mut child = root.first_child();
    while let Some(widget) = child {
        widgets.extend(descendants(&widget));
        child = widget.next_sibling();
    }
    widgets
}

fn button(root: &gtk::Widget, label: &str) -> Button {
    descendants(root)
        .into_iter()
        .filter_map(|widget| widget.downcast::<Button>().ok())
        .find(|button| {
            button.label().as_deref() == Some(label)
                || button.child().is_some_and(|child| {
                    child
                        .downcast::<gtk::Box>()
                        .ok()
                        .and_then(|row| row.last_child())
                        .and_then(|widget| widget.downcast::<Label>().ok())
                        .is_some_and(|text| text.text() == label)
                })
        })
        .unwrap()
}

#[test]
#[ignore = "requires a GNOME Wayland desktop and a fresh XDG_CONFIG_HOME"]
fn wayland_appearance_settings() {
    let path = Config::path().unwrap();
    let original = "# My settings\nfont_size = 14 # pt\nextra_option = true\n";
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    assert!(!path.exists(), "use a fresh test configuration directory");
    fs::write(&path, original).unwrap();

    let app = test_application();
    let (window, state) = create_window(&app, Config::load().unwrap());
    open_tab(&state);
    window.present();
    let terminal = active_terminal(&state).unwrap();
    let shell = shell_process(&terminal);
    let menu = descendants(window.upcast_ref())
        .into_iter()
        .find_map(|widget| widget.downcast::<MenuButton>().ok())
        .unwrap();
    let settings_button = button(menu.popover().unwrap().upcast_ref(), "Settings…");

    for (action, expected) in [("Discard changes", 14), ("Save", 18), ("Save", 12)] {
        settings_button.emit_clicked();
        let (surface, pages) = {
            let ui = state.borrow();
            let workspace = &ui.settings.as_ref().unwrap().workspace;
            assert_eq!(
                ui.stack.visible_child().as_ref(),
                Some(workspace.surface.upcast_ref())
            );
            (workspace.surface.clone(), workspace.pages.clone())
        };
        assert_eq!(pages.visible_child_name().as_deref(), Some("appearance"));
        let appearance = pages.visible_child().unwrap();
        let widgets = descendants(&appearance);
        let preview = widgets
            .iter()
            .find_map(|widget| widget.downcast_ref::<Terminal>())
            .unwrap();
        let fields = widgets
            .iter()
            .filter_map(|widget| widget.downcast_ref::<gtk::SpinButton>())
            .collect::<Vec<_>>();
        let cursor = widgets
            .iter()
            .find_map(|widget| widget.downcast_ref::<gtk::DropDown>())
            .unwrap();
        let blur = widgets
            .iter()
            .find_map(|widget| widget.downcast_ref::<gtk::Switch>())
            .unwrap();
        let opacity = widgets
            .iter()
            .find_map(|widget| widget.downcast_ref::<gtk::Scale>())
            .unwrap();
        if expected == 12 {
            button(surface.upcast_ref(), "Reset to defaults").emit_clicked();
            assert_eq!(preview.font().unwrap().size(), 12 * pango::SCALE);
            assert_eq!(preview.cursor_shape(), vte4::CursorShape::Block);
            assert_eq!(preview.margin_start(), 10);
        } else {
            blur.set_active(false);
            opacity.set_value(61.0);
            fields[0].set_value(18.0);
            fields[1].set_value(16.0);
            cursor.set_selected(1);
            assert_eq!(preview.font().unwrap().size(), 18 * pango::SCALE);
            assert_eq!(preview.cursor_shape(), vte4::CursorShape::Ibeam);
            assert_eq!(preview.margin_start(), 16);
        }
        assert!(preview.pty().is_none());
        let schemes = pages.child_by_name("schemes").unwrap();
        let list = descendants(&schemes)
            .into_iter()
            .find_map(|widget| widget.downcast::<gtk::ListBox>().ok())
            .unwrap();
        if expected != 12 {
            let sidebar = descendants(surface.upcast_ref())
                .into_iter()
                .find_map(|widget| widget.downcast::<gtk::StackSidebar>().ok())
                .unwrap();
            let navigation = descendants(sidebar.upcast_ref())
                .into_iter()
                .find_map(|widget| widget.downcast::<gtk::ListBox>().ok())
                .unwrap();
            navigation.select_row(navigation.row_at_index(1).as_ref());
            assert_eq!(pages.visible_child_name().as_deref(), Some("schemes"));
            list.select_row(list.row_at_index(1).as_ref());
            activate(&window, "next-tab");
            assert_eq!(active_terminal(&state), Some(terminal.clone()));
            activate(&window, "previous-tab");
            assert_eq!(pages.visible_child_name().as_deref(), Some("schemes"));
            assert_eq!(list.selected_row().unwrap().index(), 1);
        }
        let profiles = pages.child_by_name("profiles").unwrap();
        let profile = descendants(&profiles)
            .into_iter()
            .find_map(|widget| widget.downcast::<gtk::DropDown>().ok())
            .unwrap();
        if expected == 18 {
            profile.set_selected(1);
        }
        button(surface.upcast_ref(), action).emit_clicked();
        assert!(state.borrow().settings.is_none());
        assert_eq!(active_terminal(&state), Some(terminal.clone()));
        assert_eq!(terminal.font().unwrap().size(), expected * pango::SCALE);
        assert!(shell.exists());
        let saved = Config::load().unwrap();
        assert_eq!(saved.font_size, expected as u16);
        assert_eq!(saved.blur, expected != 18);
        assert_eq!(saved.opacity, if expected == 18 { 0.61 } else { 0.72 });
        assert_eq!(saved.padding_x, if expected == 18 { 16 } else { 10 });
        assert_eq!(
            terminal.cursor_shape(),
            if expected == 18 {
                vte4::CursorShape::Ibeam
            } else {
                vte4::CursorShape::Block
            }
        );
        let contents = fs::read_to_string(&path).unwrap();
        assert_eq!(
            saved.color_scheme,
            if expected == 14 { "" } else { "Midnight" }
        );
        assert_eq!(
            saved.default_profile,
            if expected == 14 { "" } else { "Bash" }
        );
        if action == "Discard changes" {
            assert_eq!(contents, original);
        } else {
            assert!(contents.contains(&format!("font_size = {expected} # pt")));
            assert!(contents.contains("extra_option = true"));
        }
    }
    settings_button.emit_clicked();
    let terminal_id = state.borrow().settings.as_ref().unwrap().return_to.unwrap();
    close_tab(&state, terminal_id);
    assert!(window.is_visible());
    assert!(state.borrow().tabs.is_empty());
    assert!(state.borrow().settings.is_some());
    drop(terminal);
    activate(&window, "close-tab");
    assert!(!window.is_visible());
}

#[test]
#[ignore = "requires a GNOME Wayland desktop and an isolated Bash HOME"]
fn wayland_tab_interactions() {
    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    for _ in 0..3 {
        open_tab(&state);
    }
    window.present();
    assert_eq!(state.borrow().active, Some(3));

    for (action, expected) in [
        ("next-tab", 1),
        ("next-tab", 2),
        ("previous-tab", 1),
        ("previous-tab", 3),
    ] {
        activate(&window, action);
        assert_eq!(state.borrow().active, Some(expected));
    }
    for (action, shortcut) in [
        ("win.next-tab", "<Primary>Tab"),
        ("win.previous-tab", "<Primary><Shift>Tab"),
    ] {
        let accelerators = app.accels_for_action(action);
        assert_eq!(
            gtk::accelerator_parse(&accelerators[0]),
            gtk::accelerator_parse(shortcut)
        );
    }

    let (menu, terminal, label) = {
        let ui = state.borrow();
        let tab = &ui.tabs[1];
        (
            tab.menu.clone(),
            tab.active_pane().terminal.clone(),
            tab.label.clone(),
        )
    };
    let process = shell_process(&terminal);
    menu.popup();
    wait_for("tab context menu to map", || menu.is_mapped().then_some(()));
    menu.popdown();

    for (name, expected) in [("  Build & test  ", "Build & test"), ("  ", "bash")] {
        assert!(gtk::prelude::WidgetExt::activate_action(
            &menu,
            "win.rename-tab",
            Some(&2u64.to_variant())
        )
        .is_ok());
        let dialog = wait_for("rename dialog", || {
            gtk::Window::list_toplevels()
                .into_iter()
                .find_map(|widget| {
                    widget.downcast::<gtk::Window>().ok().filter(|window| {
                        window.is_visible() && window.title().as_deref() == Some("Rename tab")
                    })
                })
        });
        let entry = dialog
            .child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast::<gtk::Entry>()
            .unwrap();
        entry.set_text(name);
        let apply = dialog
            .default_widget()
            .unwrap()
            .downcast::<Button>()
            .unwrap();
        apply.emit_clicked();
        assert_eq!(label.text(), expected);
        assert_eq!(state.borrow().active, Some(3));
        assert!(process.exists());
    }

    tabs::set_color(&state, 2, Some(gdk::RGBA::WHITE));
    let rules = state.borrow().appearance.to_string();
    assert!(rules.contains(".veyl-tab-2"));
    assert!(!rules.contains(".veyl-tab-1"));
    assert!(gtk::prelude::WidgetExt::activate_action(
        &menu,
        "win.reset-tab-color",
        Some(&2u64.to_variant())
    )
    .is_ok());
    assert!(state.borrow().tabs[1].color.is_none());
    assert!(!state
        .borrow()
        .appearance
        .to_string()
        .contains(".veyl-tab-2"));

    drop(terminal);
    menu.popup();
    assert!(gtk::prelude::WidgetExt::activate_action(
        &menu,
        "win.close-tab-at",
        Some(&2u64.to_variant())
    )
    .is_ok());
    assert!(menu.parent().is_none());
    assert_eq!(state.borrow().active, Some(3));
    wait_for("closed context tab's shell to exit", || {
        (!process.exists()).then_some(())
    });
    window.close();
}

#[test]
#[ignore = "requires a GNOME Wayland desktop and an isolated Bash HOME"]
fn wayland_search_and_tab_organization() {
    fn search_entry(search: &search::Search) -> gtk::SearchEntry {
        search
            .overlay
            .last_child()
            .unwrap()
            .first_child()
            .unwrap()
            .first_child()
            .unwrap()
            .downcast()
            .unwrap()
    }

    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    open_tab(&state);
    window.present();
    let first = active_terminal(&state).unwrap();
    let first_shell = shell_process(&first);
    let test_dir = PathBuf::from(std::env::var_os("HOME").unwrap()).join("folder with spaces");
    fs::create_dir_all(&test_dir).unwrap();
    first.feed_child(
        b"cd 'folder with spaces'; printf '[] Needle.+\\n'; printf 'LINE_%03d\\n' {1..160}\r",
    );
    wait_for("Bash to enter the folder and produce scrollback", || {
        (fs::read_link(first_shell.join("cwd")).ok().as_ref() == Some(&test_dir)
            && terminal_text(&first).contains("LINE_160"))
        .then_some(())
    });

    activate(&window, "find");
    let first_search = state.borrow().tabs[0].active_pane().search.clone();
    let first_entry = search_entry(&first_search);
    first_entry.set_text("needle.+");
    wait_for("the literal case-insensitive match in scrollback", || {
        first.has_selection().then_some(())
    });
    assert!(first.search_get_regex().is_some());
    let bar = first_search.overlay.last_child().unwrap();
    let case_button = descendants(&bar)
        .into_iter()
        .find_map(|widget| widget.downcast::<gtk::ToggleButton>().ok())
        .unwrap();
    let result = bar.last_child().unwrap().downcast::<Label>().unwrap();
    case_button.set_active(true);
    assert!(result.is_visible());
    assert_eq!(result.text(), "No matches");
    case_button.set_active(false);
    assert!(!result.is_visible());

    activate(&window, "duplicate-tab");
    let second = active_terminal(&state).unwrap();
    let second_shell = shell_process(&second);
    assert_ne!(first_shell, second_shell);
    assert_eq!(fs::read_link(second_shell.join("cwd")).unwrap(), test_dir);
    activate(&window, "find");
    let second_search = state.borrow().tabs[1].active_pane().search.clone();
    let second_entry = search_entry(&second_search);
    assert!(second_entry.text().is_empty());
    second_entry.set_text("second tab only");
    assert!(second.search_get_regex().is_some());
    second_entry.emit_by_name::<()>("stop-search", &[]);
    assert!(second.search_get_regex().is_none());
    assert!(second.has_focus());
    assert_eq!(first_entry.text(), "needle.+");

    let second_select = state.borrow().tabs[1].select.clone();
    assert!(tabs::drop_tab(&state, &second_select, 1, false));
    assert_eq!(state.borrow().tabs[0].id, 2);
    assert_eq!(
        state.borrow().tabs_bar.first_child(),
        Some(state.borrow().tabs[0].chip.clone().upcast())
    );
    activate(&window, "next-tab");
    assert_eq!(active_terminal(&state).as_ref(), Some(&first));
    assert!(first_shell.exists() && second_shell.exists());
    drop((first, second, first_search, second_search));
    window.close();
}

#[test]
#[ignore = "requires a GNOME Wayland desktop and an isolated Bash HOME"]
fn wayland_profiles_splits_and_progress() {
    let workspace = PathBuf::from(std::env::var_os("HOME").unwrap()).join("Projects folder");
    fs::create_dir_all(&workspace).unwrap();
    let mut config = Config::default();
    config.profiles.insert(
        "Projects".into(),
        crate::config::Profile {
            directory: Some(workspace.clone()),
            tab_name: Some("Project shell".into()),
            color_scheme: Some("Midnight".into()),
            ..Default::default()
        },
    );

    let app = test_application();
    let (window, state) = create_window(&app, config);
    window.present();
    gtk::prelude::WidgetExt::activate_action(
        &window,
        "win.open-profile",
        Some(&"Projects".to_variant()),
    )
    .unwrap();
    let first = active_terminal(&state).unwrap();
    let first_shell = shell_process(&first);
    assert_eq!(state.borrow().tabs[0].label.text(), "Project shell");
    assert_eq!(fs::read_link(first_shell.join("cwd")).unwrap(), workspace);
    first.feed(b"\r\n\x1b[31mTHEME_RED\x1b[0m\r\n\x1b]9;4;1;62\x1b\\");
    let progress = state.borrow().tabs[0].progress.clone();
    wait_for("profile palette and tab progress", || {
        let html = first.text_format(vte4::Format::Html).unwrap_or_default();
        (html.contains("<font color=\"#F17883\">THEME_RED</font>")
            && progress.is_visible()
            && (progress.fraction() - 0.62).abs() < 0.001)
            .then_some(())
    });
    first.feed(b"\x1b]9;4;2;62\x1b\\");
    wait_for("error progress style", || {
        progress.has_css_class("error").then_some(())
    });
    first.feed(b"\x1b]9;4;3;0\x1b\\");
    wait_for("indeterminate progress animation", || {
        state.borrow().tabs[0].progress_timer.as_ref().map(|_| ())
    });
    first.feed(b"\x1b]9;4;0;0\x1b\\");
    wait_for("completed progress to disappear", || {
        (!progress.is_visible()).then_some(())
    });

    activate(&window, "split-right");
    let right = active_terminal(&state).unwrap();
    let right_shell = shell_process(&right);
    assert_eq!(fs::read_link(right_shell.join("cwd")).unwrap(), workspace);
    assert_ne!(first_shell, right_shell);
    let surface = state.borrow().tabs[0].surface.clone();
    wait_for("side-by-side layout", || {
        first
            .compute_bounds(&surface)
            .zip(right.compute_bounds(&surface))
            .and_then(|(left, right)| (left.width() > 10.0 && right.x() > left.x()).then_some(()))
    });
    activate(&window, "focus-pane-left");
    assert_eq!(active_terminal(&state).as_ref(), Some(&first));
    activate(&window, "focus-pane-right");
    assert_eq!(active_terminal(&state).as_ref(), Some(&right));

    activate(&window, "split-down");
    let bottom = active_terminal(&state).unwrap();
    let bottom_shell = shell_process(&bottom);
    wait_for("nested vertical layout", || {
        right
            .compute_bounds(&surface)
            .zip(bottom.compute_bounds(&surface))
            .and_then(|(top, bottom)| (top.height() > 10.0 && bottom.y() > top.y()).then_some(()))
    });
    assert_eq!(state.borrow().tabs[0].panes.len(), 3);
    drop(bottom);
    activate(&window, "close-tab");
    assert_eq!(active_terminal(&state).as_ref(), Some(&right));
    wait_for("closed pane's shell to exit", || {
        (!bottom_shell.exists()).then_some(())
    });

    right.feed_child(b"exit\r");
    wait_for("exited pane to collapse", || {
        (state.borrow().tabs[0].panes.len() == 1
            && surface
                .first_child()
                .is_some_and(|child| child.is::<gtk::Overlay>()))
        .then_some(())
    });
    assert_eq!(active_terminal(&state).as_ref(), Some(&first));
    assert!(first_shell.exists());
    activate(&window, "split-right");
    let final_pane = active_terminal(&state).unwrap();
    let final_process = shell_process(&final_pane);
    drop((first, right, final_pane, surface));
    window.close();
    assert!(app.windows().is_empty());
    assert!(state.borrow().tabs.is_empty());
    drop(window);
    wait_for("window close to release both split shells", || {
        (!first_shell.exists() && !final_process.exists()).then_some(())
    });
}

#[test]
#[ignore = "requires a GNOME Wayland session and fresh settings"]
fn wayland_shortcut_editor() {
    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    window.present();
    open_tab(&state);
    open_settings(&state, "shortcuts");
    let dialog = state
        .borrow()
        .settings
        .as_ref()
        .unwrap()
        .workspace
        .surface
        .clone();
    let entries = descendants(dialog.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::Entry>().ok())
        .collect::<Vec<_>>();
    let copy = &entries[2];
    copy.set_text(&entries[3].text());
    let save = button(dialog.upcast_ref(), "Save");
    save.emit_clicked();
    assert!(dialog.is_visible());
    assert!(descendants(dialog.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<Label>().ok())
        .any(|label| label.is_visible() && label.text().contains("assigned to both")));
    assert!(state.borrow().config.shortcuts.is_empty());

    copy.set_text("<Primary><Shift>y");
    save.emit_clicked();
    assert!(dialog.parent().is_none());
    assert_eq!(
        Config::load().unwrap().shortcuts["copy"],
        vec!["<Primary><Shift>y"]
    );
    assert_eq!(
        gtk::accelerator_parse(&app.accels_for_action("win.copy")[0]),
        gtk::accelerator_parse("<Primary><Shift>y")
    );
    window.close();
}

#[test]
#[ignore = "requires a GNOME Wayland desktop and an isolated Bash HOME"]
fn wayland_window_controls() {
    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    open_tab(&state);
    window.present();
    wait_for("window to map", || window.is_mapped().then_some(()));

    let tab_controls = descendants(window.upcast_ref())
        .into_iter()
        .find(|widget| widget.has_css_class("veyl-tab-controls"))
        .unwrap();
    let (width, _, _, _) = tab_controls.measure(Orientation::Horizontal, -1);
    let (height, _, _, _) = tab_controls.measure(Orientation::Vertical, -1);
    assert_eq!((width, height), (40, 19));
    let add = tab_controls
        .first_child()
        .unwrap()
        .downcast::<Button>()
        .unwrap();
    add.emit_clicked();
    assert_eq!(state.borrow().tabs.len(), 2);
    let dropdown = tab_controls
        .last_child()
        .unwrap()
        .downcast::<MenuButton>()
        .unwrap();
    dropdown.popup();
    let popover = dropdown.popover().unwrap();
    wait_for("tab menu to open", || popover.is_mapped().then_some(()));
    dropdown.popdown();

    let controls = descendants(window.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<gtk::WindowControls>().ok())
        .find(|controls| !controls.is_empty())
        .unwrap();
    let buttons = descendants(controls.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<Button>().ok())
        .collect::<Vec<_>>();
    let names = ["minimize", "maximize", "close"];
    assert_eq!(buttons.len(), names.len());
    for (button, name) in buttons.iter().zip(names) {
        assert!(button.has_css_class(name));
        let (width, _, _, _) = button.measure(Orientation::Horizontal, -1);
        assert_eq!(width, 28);
    }

    buttons[1].emit_clicked();
    wait_for("native maximize action", || {
        window.is_maximized().then_some(())
    });
    let restore = descendants(controls.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<Button>().ok())
        .find(|button| button.has_css_class("maximize"))
        .unwrap();
    restore.emit_clicked();
    wait_for("native restore action", || {
        (!window.is_maximized()).then_some(())
    });

    let close = descendants(controls.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<Button>().ok())
        .find(|button| button.has_css_class("close"))
        .unwrap();
    close.emit_clicked();
    assert!(app.windows().is_empty());
    assert!(state.borrow().tabs.is_empty());
}

#[test]
#[ignore = "requires GNOME Wayland, an isolated Bash HOME, and fresh settings"]
fn wayland_bash_prompt() {
    let path = crate::prompt::bashrc().unwrap();
    assert!(!path.exists(), "use an isolated Bash HOME");
    let original = "alias veyl_test='printf hello'\nPROMPT_COMMAND='PS1=THEME_PROMPT'\n";
    fs::write(&path, original).unwrap();
    assert!(!Config::path().unwrap().exists(), "use fresh settings");

    let modal = |description| {
        wait_for(description, || {
            gtk::Window::list_toplevels()
                .into_iter()
                .find_map(|widget| {
                    widget
                        .downcast::<gtk::Window>()
                        .ok()
                        .filter(|dialog| dialog.is_visible() && dialog.is_modal())
                })
        })
    };

    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    open_tab(&state);
    window.present();
    let first = active_terminal(&state).unwrap();
    wait_for("existing Bash theme", || {
        terminal_text(&first).contains("THEME_PROMPT").then_some(())
    });

    open_settings(&state, "prompt");
    let (surface, page) = {
        let ui = state.borrow();
        let workspace = &ui.settings.as_ref().unwrap().workspace;
        (
            workspace.surface.clone(),
            workspace.pages.visible_child().unwrap(),
        )
    };
    let editor = descendants(&page)
        .into_iter()
        .find_map(|widget| widget.downcast::<gtk::TextView>().ok())
        .unwrap();
    let custom = "Veyl 'custom' \\w\n❯ ";
    editor.buffer().set_text(custom);
    let apply = button(&page, "Apply to .bashrc…");
    apply.emit_clicked();
    let dialog = modal("confirmation dialog");
    button(dialog.upcast_ref(), "Cancel").emit_clicked();
    assert_eq!(fs::read_to_string(&path).unwrap(), original);

    button(surface.upcast_ref(), "Save").emit_clicked();
    assert_eq!(Config::load().unwrap().bash_prompt.as_deref(), Some(custom));
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    open_tab(&state);
    let second = active_terminal(&state).unwrap();
    wait_for("custom prompt after theme hooks", || {
        let text = terminal_text(&second);
        (text.contains("Veyl 'custom' ~") && text.contains('❯')).then_some(())
    });
    second.feed_child(b"veyl_test\r");
    wait_for("alias from original .bashrc", || {
        terminal_text(&second).contains("hello").then_some(())
    });
    assert!(terminal_text(&first).contains("THEME_PROMPT"));

    open_settings(&state, "prompt");
    let page = state
        .borrow()
        .settings
        .as_ref()
        .unwrap()
        .workspace
        .pages
        .visible_child()
        .unwrap();
    button(&page, "Apply to .bashrc…").emit_clicked();
    let dialog = modal("apply confirmation");
    button(dialog.upcast_ref(), "Apply to .bashrc").emit_clicked();
    wait_for("confirmed .bashrc update", || {
        fs::read_to_string(&path)
            .unwrap()
            .contains("# BEGIN VEYL BASH PROMPT")
            .then_some(())
    });
    assert!(fs::read_to_string(&path).unwrap().starts_with(original));
    button(&page, "Shell default").emit_clicked();
    let surface = state
        .borrow()
        .settings
        .as_ref()
        .unwrap()
        .workspace
        .surface
        .clone();
    button(surface.upcast_ref(), "Save").emit_clicked();
    assert!(Config::load().unwrap().bash_prompt.is_none());
    open_tab(&state);
    let third = active_terminal(&state).unwrap();
    wait_for("prompt loaded from .bashrc alone", || {
        terminal_text(&third)
            .contains("Veyl 'custom' ~")
            .then_some(())
    });

    open_settings(&state, "prompt");
    let page = state
        .borrow()
        .settings
        .as_ref()
        .unwrap()
        .workspace
        .pages
        .visible_child()
        .unwrap();
    button(&page, "Remove Veyl prompt from .bashrc…").emit_clicked();
    let dialog = modal("remove confirmation");
    button(dialog.upcast_ref(), "Remove prompt block").emit_clicked();
    wait_for("confirmed .bashrc removal", || {
        (fs::read_to_string(&path).unwrap() == original).then_some(())
    });
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    window.close();
}

#[test]
#[ignore = "requires GNOME Wayland and an isolated Bash HOME"]
fn wayland_command_palette() {
    let app = test_application();
    let mut config = Config::default();
    config
        .shortcuts
        .insert("find".into(), vec!["<Primary><Shift>y".into()]);
    let (window, state) = create_window(&app, config);
    open_tab(&state);
    window.present();

    for (query, expected) in [
        ("in terminal find", "Find in terminal"),
        ("bash", "Open profile: Bash"),
        ("settings", "Settings"),
    ] {
        activate(&window, "command-palette");
        let dialog = wait_for("palette", || {
            gtk::Window::list_toplevels()
                .into_iter()
                .find_map(|widget| {
                    widget.downcast::<gtk::Window>().ok().filter(|window| {
                        window.is_visible() && window.title().as_deref() == Some("Command Palette")
                    })
                })
        });
        let widgets = descendants(dialog.upcast_ref());
        let search = widgets
            .iter()
            .find_map(|widget| widget.downcast_ref::<gtk::SearchEntry>())
            .unwrap();
        let list = widgets
            .iter()
            .find_map(|widget| widget.downcast_ref::<gtk::ListBox>())
            .unwrap();
        search.set_text("no such action or profile");
        wait_for("empty palette results", || {
            list.selected_row().is_none().then_some(())
        });
        search.set_text(query);
        let row = wait_for("filtered action selected", || list.selected_row());
        let labels = descendants(row.upcast_ref())
            .into_iter()
            .filter_map(|widget| widget.downcast::<Label>().ok())
            .map(|label| label.text().to_string())
            .collect::<Vec<_>>();
        assert_eq!(labels[0], expected);
        if query == "in terminal find" {
            let (key, modifiers) = gtk::accelerator_parse("<Primary><Shift>y").unwrap();
            assert_eq!(labels[1], gtk::accelerator_get_label(key, modifiers));
        }
        search.emit_by_name::<()>("activate", &[]);
        assert!(!dialog.is_visible());
    }
    assert_eq!(state.borrow().tabs.len(), 2);
    assert!(state.borrow().settings.is_some());
    window.close();
}

#[test]
#[ignore = "requires GNOME Wayland and fresh settings"]
fn wayland_config_recovery() {
    let path = Config::path().unwrap();
    assert!(!path.exists(), "use a fresh test configuration directory");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let original = "font_size = [";
    fs::write(&path, original).unwrap();

    let app = test_application();
    build(&app);
    let dialog = wait_for("configuration error", || {
        gtk::Window::list_toplevels()
            .into_iter()
            .find_map(|widget| {
                widget
                    .downcast::<gtk::Window>()
                    .ok()
                    .filter(|window| window.is_visible() && window.is_modal())
            })
    });
    let labels = descendants(dialog.upcast_ref())
        .into_iter()
        .filter_map(|widget| widget.downcast::<Label>().ok())
        .map(|label| label.text().to_string())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(labels.contains(&path.display().to_string()));
    assert!(labels.contains("font_size = ["));
    button(dialog.upcast_ref(), "Open config file");
    button(dialog.upcast_ref(), "Continue with defaults").emit_clicked();
    let window = app
        .windows()
        .into_iter()
        .find_map(|window| window.downcast::<ApplicationWindow>().ok())
        .unwrap();
    let terminal = descendants(window.upcast_ref())
        .into_iter()
        .find_map(|widget| widget.downcast::<Terminal>().ok())
        .unwrap();
    assert_eq!(terminal.font().unwrap().size(), 12 * pango::SCALE);
    assert_eq!(fs::read_to_string(&path).unwrap(), original);
    window.close();
}

#[test]
#[ignore = "requires GNOME Wayland and an isolated Bash HOME"]
fn wayland_multiline_paste_preview() {
    let app = test_application();
    let (window, state) = create_window(&app, Config::default());
    open_tab(&state);
    window.present();
    let terminal = active_terminal(&state).unwrap();
    shell_process(&terminal);
    terminal.feed_child(b"bind 'set enable-bracketed-paste off'\r");

    for (action, marker) in [("Cancel", "cancelled"), ("Paste", "accepted")] {
        let text = format!("printf '%s-%s\\n' TEST {marker}\n\n");
        paste::paste_text(&terminal, text.into());
        let dialog = wait_for("paste confirmation", || {
            gtk::Window::list_toplevels()
                .into_iter()
                .find_map(|widget| {
                    widget
                        .downcast::<gtk::Window>()
                        .ok()
                        .filter(|window| window.is_visible() && window.is_modal())
                })
        });
        button(dialog.upcast_ref(), action).emit_clicked();
    }
    wait_for("confirmed clipboard text written to PTY", || {
        terminal_text(&terminal)
            .contains("TEST-accepted")
            .then_some(())
    });
    assert!(!terminal_text(&terminal).contains("TEST-cancelled"));
    window.close();
}
