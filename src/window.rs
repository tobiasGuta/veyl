mod settings;

use crate::config::{Config, CursorShape};
use gdk4_wayland::WaylandDisplay;
use gtk::accessible::{Property, State as AccessibleState};
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, HeaderBar, Image, Label, MenuButton,
    Orientation, PolicyType, Popover, Scale, ScrolledWindow, Stack, Switch, Viewport,
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use vte4::prelude::*;
use vte4::{PtyFlags, Terminal};

struct Tab {
    id: u64,
    terminal: Terminal,
    surface: GtkBox,
    chip: GtkBox,
    select: Button,
    spawn: gio::Cancellable,
}

struct State {
    window: glib::WeakRef<ApplicationWindow>,
    tabs_bar: GtkBox,
    tabs_viewport: Viewport,
    stack: Stack,
    tabs: Vec<Tab>,
    next_id: u64,
    active: Option<u64>,
    closing: bool,
    config: Config,
    appearance: gtk::CssProvider,
    appearance_class: String,
    blur_supported: bool,
}

pub fn build(app: &Application) {
    let (window, state) = create_window(app, Config::load());
    open_tab(&state);
    window.present();
}

fn create_window(app: &Application, config: Config) -> (ApplicationWindow, Rc<RefCell<State>>) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("Veyl")
        .default_width(1060)
        .default_height(680)
        .build();
    window.add_css_class("veyl-window");
    let appearance_class = format!("veyl-window-{}", window.id());
    window.add_css_class(&appearance_class);

    let static_css = gtk::CssProvider::new();
    static_css.load_from_string(include_str!("../assets/veyl.css"));
    let appearance = gtk::CssProvider::new();
    let display = gtk::prelude::WidgetExt::display(&window);
    // GTK 4.24 forwards a backdrop filter to the Wayland compositor when available.
    let blur_supported = gtk::check_version(4, 24, 0).is_none()
        && display
            .downcast_ref::<WaylandDisplay>()
            .is_some_and(|display| display.query_registry("ext_background_effect_manager_v1"));
    gtk::style_context_add_provider_for_display(
        &display,
        &static_css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &appearance,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
    );

    let tabs_bar = GtkBox::new(Orientation::Horizontal, 2);
    tabs_bar.add_css_class("veyl-tabs");
    let tabs_viewport = Viewport::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    tabs_viewport.set_child(Some(&tabs_bar));

    let tabs_scroll = ScrolledWindow::new();
    tabs_scroll.set_policy(PolicyType::Automatic, PolicyType::Never);
    tabs_scroll.set_hexpand(false);
    tabs_scroll.set_min_content_width(100);
    tabs_scroll.set_max_content_width(640);
    tabs_scroll.set_propagate_natural_width(true);
    tabs_scroll.set_child(Some(&tabs_viewport));

    let header = HeaderBar::new();
    header.add_css_class("veyl-header");
    header.set_decoration_layout(Some(":minimize,maximize,close"));
    header.set_show_title_buttons(true);
    header.set_title_widget(Some(&GtkBox::new(Orientation::Horizontal, 0)));

    let controls = GtkBox::new(Orientation::Horizontal, 0);
    controls.add_css_class("veyl-tab-controls");
    controls.set_valign(gtk::Align::Center);

    let new_button = Button::from_icon_name("list-add-symbolic");
    new_button.set_tooltip_text(Some("New tab (Ctrl+Shift+T)"));
    new_button.set_action_name(Some("win.new-tab"));
    controls.append(&new_button);

    let menu = MenuButton::new();
    menu.set_icon_name("pan-down-symbolic");
    menu.set_tooltip_text(Some("Appearance"));
    menu.update_property(&[Property::Label("Appearance")]);
    controls.append(&menu);

    let leading = GtkBox::new(Orientation::Horizontal, 4);
    leading.append(&tabs_scroll);
    leading.append(&controls);
    header.pack_start(&leading);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    window.set_titlebar(Some(&header));
    window.set_child(Some(&stack));

    let state = Rc::new(RefCell::new(State {
        window: window.downgrade(),
        tabs_bar,
        tabs_viewport,
        stack,
        tabs: Vec::new(),
        next_id: 0,
        active: None,
        closing: false,
        config,
        appearance: appearance.clone(),
        appearance_class,
        blur_supported,
    }));

    update_appearance(&state);
    build_appearance_menu(&menu, &state);

    install_actions(&window, app, &state);
    // The window owns State. Callbacks on its actions and descendants use weak references.
    window.connect_unrealize({
        let state = state.clone();
        move |_| close_all_tabs(&state)
    });
    window.connect_destroy(move |_| {
        gtk::style_context_remove_provider_for_display(&display, &appearance);
        gtk::style_context_remove_provider_for_display(&display, &static_css);
    });
    (window, state)
}

fn update_appearance(state: &Rc<RefCell<State>>) {
    let current = state.borrow();
    let color = appearance_color(&current.config.background, "#10121a");
    let (opacity, blur) = current.config.background_style(current.blur_supported);
    let backdrop_filter = if blur {
        "backdrop-filter: blur(24px);"
    } else {
        ""
    };
    let class = &current.appearance_class;
    let css = format!(
        "window.{class} {{ background-color: rgba({},{},{},{:.3}); {backdrop_filter} }} \
         window.{class} .veyl-terminal-surface {{ padding: {}px {}px; }}",
        (color.red() * 255.0).round() as u8,
        (color.green() * 255.0).round() as u8,
        (color.blue() * 255.0).round() as u8,
        opacity,
        current.config.padding_y,
        current.config.padding_x,
    );
    let appearance = current.appearance.clone();
    drop(current);
    appearance.load_from_string(&css);
}

fn appearance_color(value: &str, fallback: &str) -> gdk::RGBA {
    let mut color = gdk::RGBA::parse(value).unwrap_or_else(|_| gdk::RGBA::parse(fallback).unwrap());
    color.set_alpha(1.0);
    color
}

fn apply_terminal_appearance(terminal: &Terminal, config: &Config) {
    terminal.set_color_foreground(&appearance_color(&config.foreground, "#e8eaf0"));
    terminal.set_cursor_shape(match config.cursor_shape {
        CursorShape::Block => vte4::CursorShape::Block,
        CursorShape::Beam => vte4::CursorShape::Ibeam,
        CursorShape::Underline => vte4::CursorShape::Underline,
    });
    let mut font = pango::FontDescription::new();
    font.set_family(&config.font_family);
    font.set_size(i32::from(config.font_size) * pango::SCALE);
    terminal.set_font(Some(&font));
}

fn apply_terminal_settings(terminal: &Terminal, config: &Config) {
    terminal.set_clear_background(false);
    terminal.set_scrollback_lines(config.scrollback_lines);
    apply_terminal_appearance(terminal, config);
}

fn open_settings(state: &Rc<RefCell<State>>) {
    let (window, original) = {
        let ui = state.borrow();
        (ui.window.upgrade(), ui.config.clone())
    };
    let Some(window) = window else { return };
    let weak_state = Rc::downgrade(state);
    settings::show(&window, &original, {
        let original = original.clone();
        move |draft| {
            let Some(state) = weak_state.upgrade() else {
                return Ok(());
            };
            draft.save_changes(&original)?;
            let terminals = {
                let mut ui = state.borrow_mut();
                ui.config = draft.clone();
                ui.tabs
                    .iter()
                    .map(|tab| tab.terminal.clone())
                    .collect::<Vec<_>>()
            };
            update_appearance(&state);
            for terminal in terminals {
                apply_terminal_appearance(&terminal, &draft);
            }
            Ok(())
        }
    });
}

fn build_appearance_menu(menu: &MenuButton, state: &Rc<RefCell<State>>) {
    let popover = Popover::new();
    popover.add_css_class("veyl-appearance");
    popover.set_has_arrow(false);
    let body = GtkBox::new(Orientation::Vertical, 12);

    let blur_row = GtkBox::new(Orientation::Horizontal, 16);
    let blur_label = Label::new(Some("Frosted background"));
    blur_label.set_xalign(0.0);
    blur_label.set_hexpand(true);
    let blur_switch = Switch::new();
    blur_switch.update_property(&[Property::Label("Frosted background")]);
    blur_row.append(&blur_label);
    blur_row.append(&blur_switch);
    body.append(&blur_row);

    let (blur_supported, blur, opacity) = {
        let ui = state.borrow();
        (ui.blur_supported, ui.config.blur, ui.config.opacity)
    };
    blur_switch.set_active(blur);
    let help = Label::new(Some(
        "Blur is unavailable. Turn it off to use transparency.",
    ));
    help.set_wrap(true);
    help.set_max_width_chars(30);
    help.set_xalign(0.0);
    help.add_css_class("veyl-hint");
    help.set_visible(blur && !blur_supported);
    body.append(&help);

    let opacity_label = Label::new(Some("Background opacity"));
    opacity_label.set_xalign(0.0);
    let slider = Scale::with_range(Orientation::Horizontal, 40.0, 100.0, 1.0);
    slider.update_property(&[Property::Label("Background opacity")]);
    slider.set_draw_value(true);
    slider.set_value_pos(gtk::PositionType::Right);
    slider.set_digits(0);
    slider.set_format_value_func(|_, value| format!("{value:.0}%"));
    slider.set_value(opacity * 100.0);
    slider.set_sensitive(!blur || blur_supported);
    slider.connect_value_changed(glib::clone!(
        #[weak]
        state,
        move |scale| {
            let previous = state.borrow().config.clone();
            state.borrow_mut().config.opacity = scale.value() / 100.0;
            save_appearance(&state, &previous);
            update_appearance(&state);
        }
    ));
    blur_switch.connect_active_notify(glib::clone!(
        #[weak]
        state,
        #[weak]
        slider,
        move |toggle| {
            let enabled = toggle.is_active();
            let previous = state.borrow().config.clone();
            state.borrow_mut().config.blur = enabled;
            help.set_visible(enabled && !blur_supported);
            slider.set_sensitive(!enabled || blur_supported);
            save_appearance(&state, &previous);
            update_appearance(&state);
        }
    ));

    body.append(&opacity_label);
    body.append(&slider);
    let settings = Button::with_label("Settings…");
    settings.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        popover,
        move |_| {
            popover.popdown();
            open_settings(&state);
        }
    ));
    body.append(&settings);
    popover.set_child(Some(&body));
    menu.set_popover(Some(&popover));
}

fn save_appearance(state: &Rc<RefCell<State>>, previous: &Config) {
    if let Err(err) = state.borrow().config.save_changes(previous) {
        eprintln!("Veyl: unable to save settings: {err}");
    }
}

fn shell_path() -> String {
    let configured = std::env::var("SHELL").unwrap_or_default();
    if configured.starts_with('/') && Path::new(&configured).is_file() {
        configured
    } else {
        "/bin/bash".to_owned()
    }
}

fn open_tab(state: &Rc<RefCell<State>>) {
    let home = std::env::var("HOME").ok();
    open_tab_with_shell(state, &shell_path(), home.as_deref());
}

fn open_tab_with_shell(state: &Rc<RefCell<State>>, shell: &str, home: Option<&str>) {
    let (id, settings, window, tabs_bar, stack) = {
        let mut ui = state.borrow_mut();
        if ui.closing {
            return;
        }
        ui.next_id += 1;
        (
            ui.next_id,
            ui.config.clone(),
            ui.window.clone(),
            ui.tabs_bar.clone(),
            ui.stack.clone(),
        )
    };

    let terminal = Terminal::new();
    terminal.set_hexpand(true);
    terminal.set_vexpand(true);
    terminal.add_css_class("veyl-terminal");
    apply_terminal_settings(&terminal, &settings);

    let surface = GtkBox::new(Orientation::Vertical, 0);
    surface.add_css_class("veyl-terminal-surface");
    surface.append(&terminal);

    let chip = GtkBox::new(Orientation::Horizontal, 0);
    chip.add_css_class("veyl-tab");

    let title = Path::new(shell)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Terminal");
    let label = Label::new(Some(title));
    label.set_ellipsize(pango::EllipsizeMode::End);
    label.set_max_width_chars(18);
    label.set_xalign(0.0);
    let icon = Image::from_icon_name("utilities-terminal-symbolic");
    icon.set_pixel_size(14);
    let tab_title = GtkBox::new(Orientation::Horizontal, 8);
    tab_title.append(&icon);
    tab_title.append(&label);
    let select = Button::new();
    select.set_child(Some(&tab_title));
    select.add_css_class("veyl-tab-select");
    select.set_tooltip_text(Some(title));
    select.update_property(&[Property::Label(&format!("Tab {id}, {title}"))]);

    let close = Button::from_icon_name("window-close-symbolic");
    close.add_css_class("veyl-tab-close");
    close.set_tooltip_text(Some("Close tab"));
    close.update_property(&[Property::Label(&format!("Close tab {id}, {title}"))]);

    chip.append(&select);
    chip.append(&close);

    select.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| activate_tab(&state, id)
    ));
    close.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| close_tab(&state, id)
    ));

    let spawn = gio::Cancellable::new();
    state.borrow_mut().tabs.push(Tab {
        id,
        terminal: terminal.clone(),
        surface: surface.clone(),
        chip: chip.clone(),
        select,
        spawn: spawn.clone(),
    });
    stack.add_child(&surface);
    tabs_bar.append(&chip);
    activate_tab(state, id);

    terminal.connect_child_exited(glib::clone!(
        #[weak]
        state,
        move |_, _| close_tab(&state, id)
    ));

    // Let VTE finish or cancel the launch even if the last window closes first.
    let hold = window
        .upgrade()
        .and_then(|window| window.application())
        .map(|app| app.hold());
    let shell_for_error = shell.to_owned();
    let weak_state = Rc::downgrade(state);
    terminal.spawn_async(
        PtyFlags::DEFAULT,
        home,
        &[shell],
        &[],
        glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        Some(&spawn),
        move |result| {
            let _hold = hold;
            let Err(err) = result else { return };
            let terminal = weak_state.upgrade().and_then(|ui| {
                ui.borrow()
                    .tabs
                    .iter()
                    .find(|tab| tab.id == id)
                    .map(|tab| tab.terminal.clone())
            });
            let Some(terminal) = terminal else {
                return;
            };
            let detail = err.message().replace(char::is_control, " ");
            let message = format!(
                "Veyl could not start the shell {shell_for_error:?}.\r\n{detail}\r\n\
                 Check that the shell is executable and $HOME is accessible.\r\n"
            );
            terminal.set_input_enabled(false);
            terminal.feed(message.as_bytes());
        },
    );
}

fn activate_tab(state: &Rc<RefCell<State>>, id: u64) {
    let (chip, select, surface, terminal, previous, stack, viewport) = {
        let mut ui = state.borrow_mut();
        let Some(tab) = ui.tabs.iter().find(|tab| tab.id == id) else {
            return;
        };
        let previous = ui
            .tabs
            .iter()
            .find(|tab| Some(tab.id) == ui.active && tab.id != id);
        let widgets = (
            tab.chip.clone(),
            tab.select.clone(),
            tab.surface.clone(),
            tab.terminal.clone(),
            previous.map(|tab| (tab.chip.clone(), tab.select.clone())),
            ui.stack.clone(),
            ui.tabs_viewport.clone(),
        );
        ui.active = Some(id);
        widgets
    };
    if let Some((chip, select)) = previous {
        chip.remove_css_class("active");
        select.update_state(&[AccessibleState::Pressed(gtk::AccessibleTristate::False)]);
    }
    stack.set_visible_child(&surface);
    chip.add_css_class("active");
    select.update_state(&[AccessibleState::Pressed(gtk::AccessibleTristate::True)]);

    let chip = chip.downgrade();
    let state = Rc::downgrade(state);
    viewport.add_tick_callback(move |viewport, _| {
        let Some(chip) = chip.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if !state
            .upgrade()
            .is_some_and(|ui| ui.borrow().active == Some(id))
        {
            return glib::ControlFlow::Break;
        }
        // Newly appended tabs need their first layout before they have scroll bounds.
        if chip.width() == 0 {
            return glib::ControlFlow::Continue;
        }
        viewport.scroll_to(&chip, None);
        glib::ControlFlow::Break
    });
    terminal.grab_focus();
}

fn close_tab(state: &Rc<RefCell<State>>, id: u64) {
    let (tab, next, window, tabs_bar, stack) = {
        let mut ui = state.borrow_mut();
        let Some(index) = ui.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let tab = ui.tabs.remove(index);
        let next = if ui.active == Some(id) {
            ui.active = None;
            ui.tabs.last().map(|tab| tab.id)
        } else {
            None
        };
        let window = if ui.tabs.is_empty() {
            ui.closing = true;
            ui.window.upgrade()
        } else {
            None
        };
        (tab, next, window, ui.tabs_bar.clone(), ui.stack.clone())
    };
    detach_tab(tab, &tabs_bar, &stack);
    if let Some(window) = window {
        window.close();
    } else if let Some(id) = next {
        activate_tab(state, id);
    }
}

fn close_all_tabs(state: &Rc<RefCell<State>>) {
    let (tabs, tabs_bar, stack) = {
        let mut ui = state.borrow_mut();
        ui.closing = true;
        ui.active = None;
        (
            std::mem::take(&mut ui.tabs),
            ui.tabs_bar.clone(),
            ui.stack.clone(),
        )
    };
    for tab in tabs {
        detach_tab(tab, &tabs_bar, &stack);
    }
}

fn detach_tab(tab: Tab, tabs_bar: &GtkBox, stack: &Stack) {
    tab.spawn.cancel();
    // Let VTE dispose the terminal, signal its child, and release the PTY.
    tabs_bar.remove(&tab.chip);
    stack.remove(&tab.surface);
}

fn active_terminal(state: &Rc<RefCell<State>>) -> Option<Terminal> {
    let ui = state.borrow();
    ui.tabs
        .iter()
        .find(|tab| Some(tab.id) == ui.active)
        .map(|tab| tab.terminal.clone())
}

fn install_actions(window: &ApplicationWindow, app: &Application, state: &Rc<RefCell<State>>) {
    let new_tab = gio::SimpleAction::new("new-tab", None);
    new_tab.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| open_tab(&state)
    ));
    window.add_action(&new_tab);

    let close = gio::SimpleAction::new("close-tab", None);
    close.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            let active = state.borrow().active;
            if let Some(id) = active {
                close_tab(&state, id);
            }
        }
    ));
    window.add_action(&close);

    let copy = gio::SimpleAction::new("copy", None);
    copy.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            if let Some(terminal) = active_terminal(&state) {
                terminal.copy_clipboard_format(vte4::Format::Text);
            }
        }
    ));
    window.add_action(&copy);

    let paste = gio::SimpleAction::new("paste", None);
    paste.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            if let Some(terminal) = active_terminal(&state) {
                terminal.paste_clipboard();
            }
        }
    ));
    window.add_action(&paste);

    app.set_accels_for_action("win.new-tab", &["<Primary><Shift>t"]);
    app.set_accels_for_action("win.close-tab", &["<Primary><Shift>w"]);
    app.set_accels_for_action("win.copy", &["<Primary><Shift>c"]);
    app.set_accels_for_action("win.paste", &["<Primary><Shift>v"]);
}

#[cfg(test)]
mod tests;
