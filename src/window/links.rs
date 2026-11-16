use super::{panes, paste, State};
use gtk::prelude::*;
use gtk::{gdk, gio, glib};
use std::{cell::RefCell, rc::Rc};
use vte4::{prelude::*, Regex, Terminal};

pub(super) fn install(terminal: &Terminal, state: &Rc<RefCell<State>>) {
    terminal.set_allow_hyperlink(true);
    // End on a URL character so common sentence punctuation is excluded.
    let pattern = r#"(?i)https?://[^\s<>"']*[^\s<>"'.,;!?)}]"#;
    let regex = Regex::for_match(pattern, 0x0000_0400).expect("built-in URL regex");
    let id = terminal.match_add_regex(&regex, 0);
    terminal.match_set_cursor_name(id, "pointer");

    let actions = gio::SimpleActionGroup::new();
    let copy = gio::SimpleAction::new("copy", None);
    copy.connect_activate(glib::clone!(
        #[weak]
        terminal,
        move |_, _| terminal.copy_clipboard_format(vte4::Format::Text)
    ));
    actions.add_action(&copy);
    let paste = gio::SimpleAction::new("paste", None);
    paste.connect_activate(glib::clone!(
        #[weak]
        terminal,
        #[weak]
        state,
        move |_, _| {
            let confirm = state.borrow().config.confirm_multiline_paste;
            paste::from_clipboard(&terminal, confirm);
        }
    ));
    actions.add_action(&paste);
    let find = gio::SimpleAction::new("find", None);
    find.connect_activate(glib::clone!(
        #[weak]
        terminal,
        #[weak]
        state,
        move |_, _| {
            let search = state
                .borrow()
                .tabs
                .iter()
                .flat_map(|tab| &tab.panes)
                .find(|pane| pane.terminal == terminal)
                .map(|pane| pane.search.clone());
            if let Some(search) = search {
                search.show(&terminal);
            }
        }
    ));
    actions.add_action(&find);
    let folder = gio::SimpleAction::new("folder", None);
    folder.connect_activate(glib::clone!(
        #[weak]
        terminal,
        #[weak]
        state,
        move |_, _| {
            let pid = state
                .borrow()
                .tabs
                .iter()
                .flat_map(|tab| &tab.panes)
                .find(|pane| pane.terminal == terminal)
                .and_then(|pane| pane.pid);
            let Some(path) = panes::current_directory(&terminal, pid) else {
                return;
            };
            let Some(window) = terminal.root().and_downcast::<gtk::Window>() else {
                return;
            };
            let file = gio::File::for_path(path);
            gtk::FileLauncher::new(Some(&file)).launch(
                Some(&window),
                gio::Cancellable::NONE,
                report_error(&window),
            );
        }
    ));
    actions.add_action(&folder);

    for (name, open) in [("open-link", true), ("copy-link", false)] {
        let action = gio::SimpleAction::new(name, Some(&String::static_variant_type()));
        action.connect_activate(glib::clone!(
            #[weak]
            terminal,
            move |_, target| {
                let Some(value) = target
                    .and_then(glib::Variant::str)
                    .filter(|uri| is_web_uri(uri))
                else {
                    return;
                };
                if open {
                    open_link(&terminal, value);
                } else {
                    terminal.clipboard().set_text(value);
                }
            }
        ));
        actions.add_action(&action);
    }
    terminal.insert_action_group("terminal", Some(&actions));
    terminal.set_context_menu_model(Some(&context_menu(None)));

    let secondary = gtk::GestureClick::new();
    secondary.set_button(gdk::BUTTON_SECONDARY);
    secondary.set_propagation_phase(gtk::PropagationPhase::Capture);
    secondary.connect_pressed(glib::clone!(
        #[weak]
        terminal,
        move |_, _, x, y| {
            terminal
                .set_context_menu_model(Some(&context_menu(link_at(&terminal, x, y).as_deref())));
        }
    ));
    terminal.add_controller(secondary);

    let keyboard = gtk::EventControllerKey::new();
    keyboard.set_propagation_phase(gtk::PropagationPhase::Capture);
    keyboard.connect_key_pressed(glib::clone!(
        #[weak]
        terminal,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, modifiers| {
            if key == gdk::Key::Menu
                || (key == gdk::Key::F10 && modifiers.contains(gdk::ModifierType::SHIFT_MASK))
            {
                terminal.set_context_menu_model(Some(&context_menu(None)));
            }
            glib::Propagation::Proceed
        }
    ));
    terminal.add_controller(keyboard);

    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_PRIMARY);
    click.set_propagation_phase(gtk::PropagationPhase::Capture);
    click.connect_pressed(glib::clone!(
        #[weak]
        terminal,
        move |gesture, count, x, y| {
            let modifiers = gesture.current_event_state() & gtk::accelerator_get_default_mod_mask();
            if count == 1 && modifiers == gdk::ModifierType::CONTROL_MASK {
                if let Some(link) = link_at(&terminal, x, y) {
                    gesture.set_state(gtk::EventSequenceState::Claimed);
                    open_link(&terminal, &link);
                }
            }
        }
    ));
    terminal.add_controller(click);
}

fn context_menu(link: Option<&str>) -> gio::Menu {
    let menu = gio::Menu::new();
    if let Some(link) = link {
        let section = gio::Menu::new();
        for (label, action) in [
            ("Open link", "terminal.open-link"),
            ("Copy link", "terminal.copy-link"),
        ] {
            let item = gio::MenuItem::new(Some(label), None);
            item.set_action_and_target_value(Some(action), Some(&link.to_variant()));
            section.append_item(&item);
        }
        menu.append_section(None, &section);
    }
    let section = gio::Menu::new();
    for (label, action) in [
        ("Copy", "terminal.copy"),
        ("Paste", "terminal.paste"),
        ("Find in terminal", "terminal.find"),
        ("Open current folder", "terminal.folder"),
    ] {
        section.append(Some(label), Some(action));
    }
    menu.append_section(None, &section);
    menu
}

pub(super) fn link_at(terminal: &Terminal, x: f64, y: f64) -> Option<glib::GString> {
    terminal
        .check_hyperlink_at(x, y)
        .or_else(|| terminal.check_match_at(x, y).0)
        .filter(|uri| is_web_uri(uri))
}

fn is_web_uri(value: &str) -> bool {
    !value.contains(char::is_control)
        && glib::Uri::parse(value, glib::UriFlags::NONE).is_ok_and(|uri| {
            matches!(uri.scheme().as_str(), "http" | "https")
                && uri.host().is_some_and(|host| !host.is_empty())
        })
}

fn open_link(terminal: &Terminal, link: &str) {
    let Some(window) = terminal.root().and_downcast::<gtk::Window>() else {
        return;
    };
    gtk::UriLauncher::new(link).launch(
        Some(&window),
        gio::Cancellable::NONE,
        report_error(&window),
    );
}

fn report_error(window: &gtk::Window) -> impl FnOnce(Result<(), glib::Error>) + 'static {
    glib::clone!(
        #[weak]
        window,
        move |result: Result<(), glib::Error>| {
            if let Err(error) = result {
                if !error.matches(gtk::DialogError::Dismissed) {
                    gtk::AlertDialog::builder()
                        .message("Could not open the destination")
                        .detail(error.to_string())
                        .build()
                        .show(Some(&window));
                }
            }
        }
    )
}

#[cfg(test)]
mod tests {
    #[test]
    fn permits_only_http_and_https_with_hosts() {
        for uri in [
            "https://example.org/docs",
            "http://localhost:8080",
            "HTTPS://example.org/",
        ] {
            assert!(super::is_web_uri(uri), "{uri}");
        }
        for uri in [
            "file:///etc/passwd",
            "javascript:alert(1)",
            "http:example.org",
            "https:///test",
            "https://example.org\n",
        ] {
            assert!(!super::is_web_uri(uri), "{uri}");
        }
    }
}
