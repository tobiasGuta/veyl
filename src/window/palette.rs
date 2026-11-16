use super::shortcuts;
use gtk::prelude::*;
use gtk::{gdk, gio, glib, ApplicationWindow};
use std::collections::BTreeMap;

struct Command {
    label: String,
    action: String,
    target: Option<glib::Variant>,
}

pub(super) fn show<T>(parent: &ApplicationWindow, profiles: &BTreeMap<String, T>) {
    let Some(app) = parent.application() else {
        return;
    };
    let commands = shortcuts::ACTIONS
        .iter()
        .filter(|(id, _, _)| *id != "command-palette")
        .map(|(id, label, _)| Command {
            label: (*label).into(),
            action: (*id).into(),
            target: None,
        })
        .chain(profiles.keys().map(|name| Command {
            label: format!("Open profile: {name}"),
            action: "open-profile".into(),
            target: Some(name.to_variant()),
        }))
        .filter(|command| parent.is_action_enabled(&command.action))
        .collect::<Vec<_>>();

    let dialog = gtk::Window::builder()
        .title("Command Palette")
        .transient_for(parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(480)
        .default_height(380)
        .build();
    dialog.set_titlebar(Some(&gtk::HeaderBar::new()));
    let body = gtk::Box::new(gtk::Orientation::Vertical, 8);
    body.set_margin_top(10);
    body.set_margin_bottom(10);
    body.set_margin_start(12);
    body.set_margin_end(12);

    let search = gtk::SearchEntry::new();
    search.set_placeholder_text(Some("Search Veyl actions and profiles"));
    body.append(&search);

    let list = gtk::ListBox::new();
    list.add_css_class("veyl-palette-list");
    for command in &commands {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let label = gtk::Label::builder()
            .label(&command.label)
            .xalign(0.0)
            .hexpand(true)
            .ellipsize(gtk::pango::EllipsizeMode::End)
            .build();
        row.append(&label);
        let keys = app
            .accels_for_action(&format!("win.{}", command.action))
            .iter()
            .filter_map(gtk::accelerator_parse)
            .map(|(key, modifiers)| gtk::accelerator_get_label(key, modifiers).to_string())
            .collect::<Vec<_>>()
            .join(" / ");
        if !keys.is_empty() {
            let keys = gtk::Label::new(Some(&keys));
            keys.add_css_class("dim-label");
            row.append(&keys);
        }
        list.append(&row);
    }
    let empty = gtk::Label::new(Some("No matching actions"));
    empty.add_css_class("dim-label");
    list.set_placeholder(Some(&empty));
    let viewport = gtk::Viewport::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    viewport.set_child(Some(&list));
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .vexpand(true)
        .child(&viewport)
        .build();
    body.append(&scroll);
    dialog.set_child(Some(&body));

    let labels = commands
        .iter()
        .map(|command| command.label.to_lowercase())
        .collect::<Vec<_>>();
    list.set_filter_func(glib::clone!(
        #[weak]
        search,
        #[upgrade_or]
        true,
        move |row| {
            let query = search.text().to_lowercase();
            query
                .split_whitespace()
                .all(|word| labels[row.index() as usize].contains(word))
        }
    ));
    search.set_search_delay(0);
    search.connect_search_changed(glib::clone!(
        #[weak]
        list,
        move |_| {
            list.invalidate_filter();
            list.select_row(first_match(&list).as_ref());
        }
    ));
    search.connect_activate(glib::clone!(
        #[weak]
        list,
        move |_| {
            if let Some(row) = list.selected_row().filter(|row| row.is_child_visible()) {
                list.emit_by_name::<()>("row-activated", &[&row]);
            }
        }
    ));
    search.connect_stop_search(glib::clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));
    list.connect_row_activated(glib::clone!(
        #[weak]
        parent,
        #[weak]
        dialog,
        move |_, row| {
            let command = &commands[row.index() as usize];
            dialog.close();
            gio::prelude::ActionGroupExt::activate_action(
                &parent,
                &command.action,
                command.target.as_ref(),
            );
        }
    ));

    let keyboard = gtk::EventControllerKey::new();
    keyboard.set_propagation_phase(gtk::PropagationPhase::Capture);
    keyboard.connect_key_pressed(glib::clone!(
        #[weak]
        list,
        #[weak]
        dialog,
        #[weak]
        viewport,
        #[weak]
        search,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, modifiers| {
            if key == gdk::Key::Escape {
                dialog.close();
                return glib::Propagation::Stop;
            }
            if !search.delegate().is_some_and(|entry| entry.has_focus())
                || modifiers.intersects(gtk::accelerator_get_default_mod_mask())
            {
                return glib::Propagation::Proceed;
            }
            let step = match key {
                gdk::Key::Up => -1,
                gdk::Key::Down => 1,
                _ => return glib::Propagation::Proceed,
            };
            let target = list.selected_row().and_then(|row| {
                let mut index = row.index() + step;
                while let Some(candidate) = list.row_at_index(index) {
                    if candidate.is_child_visible() {
                        return Some(candidate);
                    }
                    index += step;
                }
                None
            });
            if let Some(target) = target
                .or_else(|| list.selected_row())
                .or_else(|| first_match(&list))
            {
                list.select_row(Some(&target));
                viewport.scroll_to(&target, None);
            }
            glib::Propagation::Stop
        }
    ));
    dialog.add_controller(keyboard);
    list.select_row(first_match(&list).as_ref());
    dialog.present();
    search.grab_focus();
}

fn first_match(list: &gtk::ListBox) -> Option<gtk::ListBoxRow> {
    (0..)
        .map_while(|index| list.row_at_index(index))
        .find(|row| row.is_child_visible())
}
