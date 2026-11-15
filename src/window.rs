use crate::config::Config;
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
    chip: GtkBox,
    select: Button,
}

struct State {
    window: ApplicationWindow,
    tabs_bar: GtkBox,
    tabs_viewport: Viewport,
    stack: Stack,
    tabs: Vec<Tab>,
    next_id: u64,
    active: Option<u64>,
    config: Config,
    appearance: gtk::CssProvider,
    blur_supported: bool,
}

pub fn build(app: &Application) {
    let config = Config::load();

    let window = ApplicationWindow::builder()
        .application(app)
        .title("Veyl")
        .default_width(1060)
        .default_height(680)
        .build();
    window.add_css_class("veyl-window");

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
        window: window.clone(),
        tabs_bar,
        tabs_viewport,
        stack,
        tabs: Vec::new(),
        next_id: 0,
        active: None,
        config,
        appearance,
        blur_supported,
    }));

    update_appearance(&state);
    build_appearance_menu(&menu, &state);

    install_actions(&window, app, &state);
    open_tab(&state);
    window.present();
}

fn update_appearance(state: &Rc<RefCell<State>>) {
    let current = state.borrow();
    let color = gdk::RGBA::parse(&current.config.background)
        .unwrap_or_else(|_| gdk::RGBA::new(0.063, 0.071, 0.102, 1.0));
    let (opacity, blur) = current.config.background_style(current.blur_supported);
    let backdrop_filter = if blur {
        "backdrop-filter: blur(24px);"
    } else {
        ""
    };
    let css = format!(
        "window.veyl-window {{ background-color: rgba({},{},{},{:.3}); {backdrop_filter} }}",
        (color.red() * 255.0).round() as u8,
        (color.green() * 255.0).round() as u8,
        (color.blue() * 255.0).round() as u8,
        opacity,
    );
    current.appearance.load_from_string(&css);
}

fn apply_terminal_settings(terminal: &Terminal, config: &Config) {
    let fallback = gdk::RGBA::new(0.91, 0.92, 0.94, 1.0);
    let mut foreground = gdk::RGBA::parse(&config.foreground).unwrap_or(fallback);
    foreground.set_alpha(1.0);
    terminal.set_color_foreground(&foreground);
    terminal.set_clear_background(false);
    terminal.set_scrollback_lines(config.scrollback_lines);
    let font = pango::FontDescription::from_string(&format!(
        "{} {}",
        config.font_family, config.font_size
    ));
    terminal.set_font(Some(&font));
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
    {
        let state = state.clone();
        slider.connect_value_changed(move |scale| {
            state.borrow_mut().config.opacity = scale.value() / 100.0;
            save_appearance(&state);
            update_appearance(&state);
        });
    }
    {
        let state = state.clone();
        let slider = slider.clone();
        blur_switch.connect_active_notify(move |toggle| {
            let enabled = toggle.is_active();
            state.borrow_mut().config.blur = enabled;
            help.set_visible(enabled && !blur_supported);
            slider.set_sensitive(!enabled || blur_supported);
            save_appearance(&state);
            update_appearance(&state);
        });
    }

    body.append(&opacity_label);
    body.append(&slider);
    popover.set_child(Some(&body));
    menu.set_popover(Some(&popover));
}

fn save_appearance(state: &Rc<RefCell<State>>) {
    if let Err(err) = state.borrow().config.save() {
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
    let (id, settings) = {
        let mut ui = state.borrow_mut();
        ui.next_id += 1;
        (ui.next_id, ui.config.clone())
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

    let shell = shell_path();
    let title = Path::new(&shell)
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

    {
        let state = state.clone();
        select.connect_clicked(move |_| activate_tab(&state, id));
    }
    {
        let state = state.clone();
        close.connect_clicked(move |_| close_tab(&state, id));
    }

    {
        let mut ui = state.borrow_mut();
        ui.tabs_bar.append(&chip);
        ui.stack.add_named(&surface, Some(&format!("tab-{id}")));
        ui.tabs.push(Tab {
            id,
            terminal: terminal.clone(),
            chip,
            select,
        });
    }
    activate_tab(state, id);

    {
        let state = state.clone();
        terminal.connect_child_exited(move |_, _status| close_tab(&state, id));
    }

    let shell_for_error = shell.clone();
    let home = std::env::var("HOME").ok();
    terminal.spawn_async(
        PtyFlags::DEFAULT,
        home.as_deref(),
        &[shell.as_str()],
        &[],
        glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        None::<&gio::Cancellable>,
        move |result| {
            if let Err(err) = result {
                eprintln!("Veyl: failed to start shell {shell_for_error}: {err}");
            }
        },
    );
}

fn activate_tab(state: &Rc<RefCell<State>>, id: u64) {
    let mut ui = state.borrow_mut();
    let Some(selected) = ui.tabs.iter().find(|tab| tab.id == id) else {
        return;
    };
    let chip = selected.chip.downgrade();
    let terminal = selected.terminal.clone();
    ui.active = Some(id);
    ui.stack.set_visible_child_name(&format!("tab-{id}"));
    for tab in &ui.tabs {
        let active = tab.id == id;
        let pressed = if active {
            gtk::AccessibleTristate::True
        } else {
            gtk::AccessibleTristate::False
        };
        tab.select
            .update_state(&[AccessibleState::Pressed(pressed)]);
        if active {
            tab.chip.add_css_class("active");
        } else {
            tab.chip.remove_css_class("active");
        }
    }
    let state = Rc::downgrade(state);
    ui.tabs_viewport.add_tick_callback(move |viewport, _| {
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
    drop(ui);
    terminal.grab_focus();
}

fn close_tab(state: &Rc<RefCell<State>>, id: u64) {
    let (next, should_close) = {
        let mut ui = state.borrow_mut();
        let Some(index) = ui.tabs.iter().position(|tab| tab.id == id) else {
            return;
        };
        let tab = ui.tabs.remove(index);
        ui.tabs_bar.remove(&tab.chip);
        // The GTK Stack owns the surface. The VTE widget will be released with it,
        // closing the PTY; the child may receive SIGHUP (not guaranteed).
        if let Some(surface) = ui.stack.child_by_name(&format!("tab-{id}")) {
            ui.stack.remove(&surface);
        }
        let next = if ui.active == Some(id) {
            ui.tabs.last().map(|tab| tab.id)
        } else {
            None
        };
        if ui.active == Some(id) {
            ui.active = next;
        }
        (next, ui.tabs.is_empty())
    };
    if should_close {
        state.borrow().window.close();
    } else if let Some(id) = next {
        activate_tab(state, id);
    }
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
    {
        let state = state.clone();
        new_tab.connect_activate(move |_, _| open_tab(&state));
    }
    window.add_action(&new_tab);

    let close = gio::SimpleAction::new("close-tab", None);
    {
        let state = state.clone();
        close.connect_activate(move |_, _| {
            let active_id = state.borrow().active;
            if let Some(id) = active_id {
                close_tab(&state, id);
            }
        });
    }
    window.add_action(&close);

    let copy = gio::SimpleAction::new("copy", None);
    {
        let state = state.clone();
        copy.connect_activate(move |_, _| {
            if let Some(terminal) = active_terminal(&state) {
                terminal.copy_clipboard_format(vte4::Format::Text);
            }
        });
    }
    window.add_action(&copy);

    let paste = gio::SimpleAction::new("paste", None);
    {
        let state = state.clone();
        paste.connect_activate(move |_, _| {
            if let Some(terminal) = active_terminal(&state) {
                terminal.paste_clipboard();
            }
        });
    }
    window.add_action(&paste);

    app.set_accels_for_action("win.new-tab", &["<Primary><Shift>t"]);
    app.set_accels_for_action("win.close-tab", &["<Primary><Shift>w"]);
    app.set_accels_for_action("win.copy", &["<Primary><Shift>c"]);
    app.set_accels_for_action("win.paste", &["<Primary><Shift>v"]);
}
