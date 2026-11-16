use gtk::accessible::Property;
use gtk::prelude::*;
use gtk::{
    glib, Align, Box as GtkBox, Button, Label, Orientation, Overlay, SearchEntry, ToggleButton,
};
use vte4::{prelude::*, Regex, Terminal};

// VTE searches terminal rows with PCRE2 multiline semantics.
const PCRE2_MULTILINE: u32 = 0x0000_0400;

#[derive(Clone)]
pub(super) struct Search {
    pub(super) overlay: Overlay,
    bar: GtkBox,
    entry: SearchEntry,
    case: ToggleButton,
    status: Label,
}

impl Search {
    pub(super) fn new(terminal: &Terminal) -> Self {
        let overlay = Overlay::new();
        overlay.set_child(Some(terminal));

        let bar = GtkBox::new(Orientation::Vertical, 4);
        bar.add_css_class("veyl-find");
        bar.set_halign(Align::End);
        bar.set_valign(Align::Start);
        bar.set_margin_top(4);
        bar.set_margin_end(4);
        bar.set_visible(false);
        overlay.add_overlay(&bar);

        let row = GtkBox::new(Orientation::Horizontal, 4);
        let entry = SearchEntry::new();
        entry.set_placeholder_text(Some("Find in terminal"));
        entry.update_property(&[Property::Label("Find in terminal")]);
        row.append(&entry);

        let case = ToggleButton::with_label("Aa");
        case.set_tooltip_text(Some("Match case"));
        case.update_property(&[Property::Label("Match case")]);
        row.append(&case);

        let status = Label::new(None);
        status.set_visible(false);
        status.set_xalign(0.0);
        status.update_property(&[Property::Label("Search result")]);

        for (icon, label, forward) in [
            ("go-up-symbolic", "Previous match", false),
            ("go-down-symbolic", "Next match", true),
        ] {
            let button = Button::from_icon_name(icon);
            button.set_tooltip_text(Some(label));
            button.update_property(&[Property::Label(label)]);
            button.connect_clicked(glib::clone!(
                #[weak]
                terminal,
                #[weak]
                status,
                move |_| {
                    find(&terminal, &status, forward);
                }
            ));
            row.append(&button);
        }

        let close = Button::from_icon_name("window-close-symbolic");
        close.set_tooltip_text(Some("Close search (Esc)"));
        close.update_property(&[Property::Label("Close search")]);
        close.connect_clicked(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            bar,
            move |_| {
                dismiss(&terminal, &bar);
            }
        ));
        row.append(&close);

        entry.connect_changed(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            status,
            #[weak]
            case,
            #[weak]
            bar,
            move |entry| {
                if bar.is_visible() {
                    update(&terminal, &status, &entry.text(), case.is_active());
                }
            }
        ));
        case.connect_toggled(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            status,
            #[weak]
            entry,
            move |case| update(&terminal, &status, &entry.text(), case.is_active())
        ));
        entry.connect_activate(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            status,
            move |_| {
                find(&terminal, &status, true);
            }
        ));
        entry.connect_next_match(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            status,
            move |_| {
                find(&terminal, &status, true);
            }
        ));
        entry.connect_previous_match(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            status,
            move |_| {
                find(&terminal, &status, false);
            }
        ));
        entry.connect_stop_search(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            bar,
            move |_| {
                dismiss(&terminal, &bar);
            }
        ));
        let escape = gtk::EventControllerKey::new();
        escape.set_propagation_phase(gtk::PropagationPhase::Capture);
        escape.connect_key_pressed(glib::clone!(
            #[weak]
            terminal,
            #[weak]
            bar,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gtk::gdk::Key::Escape {
                    dismiss(&terminal, &bar);
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        bar.add_controller(escape);
        bar.append(&row);
        bar.append(&status);
        Self {
            overlay,
            bar,
            entry,
            case,
            status,
        }
    }

    pub(super) fn show(&self, terminal: &Terminal) {
        if !self.bar.is_visible() {
            self.bar.set_visible(true);
            update(
                terminal,
                &self.status,
                &self.entry.text(),
                self.case.is_active(),
            );
        }
        self.entry.grab_focus();
        self.entry.select_region(0, -1);
    }
}

fn dismiss(terminal: &Terminal, bar: &GtkBox) {
    bar.set_visible(false);
    terminal.search_set_regex(None, 0);
    terminal.grab_focus();
}

fn update(terminal: &Terminal, status: &Label, query: &str, case_sensitive: bool) {
    terminal.search_set_regex(None, 0);
    status.set_visible(false);
    if query.is_empty() {
        return;
    }
    if query.len() > 4096 {
        status.set_text("Search text is too long");
        status.set_visible(true);
        return;
    }
    let prefix = if case_sensitive { "" } else { "(?i)" };
    let mut pattern = String::from(prefix);
    for character in query.chars() {
        if r"\.^$|?*+()[]{}".contains(character) {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    match Regex::for_search(&pattern, PCRE2_MULTILINE) {
        Ok(regex) => {
            terminal.search_set_regex(Some(&regex), 0);
            terminal.search_set_wrap_around(true);
            find(terminal, status, false);
        }
        Err(_) => {
            status.set_text("Search text is too long");
            status.set_visible(true);
        }
    }
}

fn find(terminal: &Terminal, status: &Label, forward: bool) {
    if terminal.search_get_regex().is_none() {
        return;
    }
    let found = if forward {
        terminal.search_find_next()
    } else {
        terminal.search_find_previous()
    };
    status.set_text("No matches");
    status.set_visible(!found);
}
