use super::{activate_tab, close_tab, update_appearance, State};
use gtk::accessible::Property;
use gtk::prelude::*;
use gtk::{gdk, gio, glib, Application, ApplicationWindow, Button, PopoverMenu};
use std::cell::RefCell;
use std::fmt::Write;
use std::rc::Rc;

pub(super) fn context_menu(tab: &Button, id: u64) -> PopoverMenu {
    let model = gio::Menu::new();
    for (label, action) in [
        ("Rename tab…", "win.rename-tab"),
        ("Tab color…", "win.tab-color"),
        ("Reset tab color", "win.reset-tab-color"),
        ("Close tab", "win.close-tab-at"),
    ] {
        let item = gio::MenuItem::new(Some(label), None);
        item.set_action_and_target_value(Some(action), Some(&id.to_variant()));
        model.append_item(&item);
    }

    let menu = PopoverMenu::from_model(Some(&model));
    menu.set_has_arrow(false);
    menu.add_css_class("veyl-tab-menu");
    menu.set_parent(tab);

    let click = gtk::GestureClick::new();
    click.set_button(gdk::BUTTON_SECONDARY);
    click.connect_pressed(glib::clone!(
        #[weak]
        menu,
        move |gesture, _, x, y| {
            gesture.set_state(gtk::EventSequenceState::Claimed);
            menu.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
            menu.popup();
        }
    ));
    tab.add_controller(click);

    let keys = gtk::EventControllerKey::new();
    keys.connect_key_pressed(glib::clone!(
        #[weak]
        menu,
        #[upgrade_or]
        glib::Propagation::Proceed,
        move |_, key, _, modifiers| {
            let modifiers = modifiers & gtk::accelerator_get_default_mod_mask();
            if (key == gdk::Key::Menu && modifiers.is_empty())
                || (key == gdk::Key::F10 && modifiers == gdk::ModifierType::SHIFT_MASK)
            {
                menu.set_pointing_to(None);
                menu.popup();
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        }
    ));
    tab.add_controller(keys);
    menu
}

pub(super) fn install_actions(
    window: &ApplicationWindow,
    app: &Application,
    state: &Rc<RefCell<State>>,
) {
    type TabAction = fn(&Rc<RefCell<State>>, u64);
    let actions: [(&str, TabAction); 4] = [
        ("rename-tab", open_rename),
        ("tab-color", choose_color),
        ("reset-tab-color", |state, id| set_color(state, id, None)),
        ("close-tab-at", close_tab),
    ];
    for (name, callback) in actions {
        let action = gio::SimpleAction::new(name, Some(&u64::static_variant_type()));
        action.connect_activate(glib::clone!(
            #[weak]
            state,
            move |_, parameter| {
                if let Some(id) = parameter.and_then(glib::Variant::get::<u64>) {
                    callback(&state, id);
                }
            }
        ));
        window.add_action(&action);
    }

    for (name, shortcut, forward) in [
        ("next-tab", "<Primary>Tab", true),
        ("previous-tab", "<Primary><Shift>Tab", false),
    ] {
        let action = gio::SimpleAction::new(name, None);
        action.connect_activate(glib::clone!(
            #[weak]
            state,
            move |_, _| {
                cycle_tab(&state, forward);
            }
        ));
        window.add_action(&action);
        app.set_accels_for_action(&format!("win.{name}"), &[shortcut]);
    }
}

fn cycle_tab(state: &Rc<RefCell<State>>, forward: bool) {
    let id = {
        let ui = state.borrow();
        let Some(current) = ui.tabs.iter().position(|tab| Some(tab.id) == ui.active) else {
            return;
        };
        let count = ui.tabs.len();
        let next = if forward {
            (current + 1) % count
        } else {
            (current + count - 1) % count
        };
        ui.tabs[next].id
    };
    activate_tab(state, id);
}

fn open_rename(state: &Rc<RefCell<State>>, id: u64) {
    let (parent, label, default_title) = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| tab.id == id) else {
            return;
        };
        (
            ui.window.upgrade(),
            tab.label.clone(),
            tab.default_title.clone(),
        )
    };
    let Some(parent) = parent else { return };
    let dialog = gtk::Window::builder()
        .title("Rename tab")
        .transient_for(&parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(360)
        .resizable(false)
        .build();
    let entry = gtk::Entry::builder()
        .max_length(128)
        .placeholder_text(&default_title)
        .activates_default(true)
        .build();
    entry.update_property(&[Property::Label("Tab name")]);
    if label.text() != default_title {
        entry.set_text(&label.text());
    }
    let content = gtk::Box::new(gtk::Orientation::Vertical, 12);
    content.set_margin_top(16);
    content.set_margin_bottom(16);
    content.set_margin_start(16);
    content.set_margin_end(16);
    content.append(&entry);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::End);
    let cancel = Button::with_label("Cancel");
    cancel.connect_clicked(glib::clone!(
        #[weak]
        dialog,
        move |_| dialog.close()
    ));
    actions.append(&cancel);
    let apply = Button::with_label("Rename");
    apply.add_css_class("suggested-action");
    apply.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        dialog,
        #[weak]
        entry,
        move |_| {
            rename(&state, id, entry.text().trim());
            dialog.close();
        }
    ));
    actions.append(&apply);
    content.append(&actions);
    dialog.set_child(Some(&content));
    dialog.set_default_widget(Some(&apply));
    dialog.present();
    entry.grab_focus();
    entry.select_region(0, -1);
}

fn rename(state: &Rc<RefCell<State>>, id: u64, title: &str) {
    let (label, select, close, default_title) = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| tab.id == id) else {
            return;
        };
        (
            tab.label.clone(),
            tab.select.clone(),
            tab.close.clone(),
            tab.default_title.clone(),
        )
    };
    let title = if title.is_empty() {
        &default_title
    } else {
        title
    };
    label.set_text(title);
    select.set_tooltip_text(Some(title));
    select.update_property(&[Property::Label(&format!("Tab {id}, {title}"))]);
    close.update_property(&[Property::Label(&format!("Close tab {id}, {title}"))]);
}

fn choose_color(state: &Rc<RefCell<State>>, id: u64) {
    let (parent, color) = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| tab.id == id) else {
            return;
        };
        (ui.window.upgrade(), tab.color)
    };
    let Some(parent) = parent else { return };
    gtk::ColorDialog::builder()
        .title("Tab color")
        .with_alpha(false)
        .build()
        .choose_rgba(
            Some(&parent),
            color.as_ref(),
            gio::Cancellable::NONE,
            glib::clone!(
                #[weak]
                state,
                move |result| {
                    if let Ok(color) = result {
                        set_color(&state, id, Some(color));
                    }
                }
            ),
        );
}

pub(super) fn set_color(state: &Rc<RefCell<State>>, id: u64, color: Option<gdk::RGBA>) {
    {
        let mut ui = state.borrow_mut();
        let Some(tab) = ui.tabs.iter_mut().find(|tab| tab.id == id) else {
            return;
        };
        tab.color = color;
    }
    update_appearance(state);
}

pub(super) fn append_color_rules(css: &mut String, state: &State) {
    for tab in &state.tabs {
        if let Some(color) = tab.color {
            let linear = |channel: f32| {
                if channel <= 0.04045 {
                    channel / 12.92
                } else {
                    ((channel + 0.055) / 1.055).powf(2.4)
                }
            };
            let luminance = 0.2126 * linear(color.red())
                + 0.7152 * linear(color.green())
                + 0.0722 * linear(color.blue());
            let foreground = if luminance > 0.179 { "#000" } else { "#fff" };
            write!(
                css,
                " window.{} .veyl-tab-{} {{ background-color: {}; color: {foreground}; }}",
                state.appearance_class,
                tab.id,
                color.to_str(),
            )
            .expect("writing CSS to a String");
        }
    }
}
