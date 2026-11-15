use crate::config::Config;
use gtk::gdk;
use gtk::gio;
use gtk::glib;
use gtk::pango;
use gtk::prelude::*;
use gtk::{
    Application, ApplicationWindow, Box as GtkBox, Button, HeaderBar, Label, MenuButton,
    Orientation, PolicyType, Popover, Scale, ScrolledWindow, Stack,
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
    stack: Stack,
    tabs: Vec<Tab>,
    next_id: u64,
    active: Option<u64>,
    config: Config,
    appearance: gtk::CssProvider,
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
    static_css.load_from_data(include_str!("../assets/veyl.css"));
    let appearance = gtk::CssProvider::new();
    let display = gtk::prelude::WidgetExt::display(&window);
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

    let tabs_bar = GtkBox::new(Orientation::Horizontal, 4);
    tabs_bar.add_css_class("veyl-tabs");

    let tabs_scroll = ScrolledWindow::new();
    tabs_scroll.set_policy(PolicyType::Automatic, PolicyType::Never);
    tabs_scroll.set_hexpand(true);
    tabs_scroll.set_min_content_width(180);
    tabs_scroll.set_child(Some(&tabs_bar));

    let heading = GtkBox::new(Orientation::Horizontal, 8);
    heading.set_hexpand(true);
    let logo = Label::new(Some("›_"));
    logo.add_css_class("veyl-brand");
    heading.append(&logo);
    heading.append(&tabs_scroll);

    let header = HeaderBar::new();
    header.add_css_class("veyl-header");
    header.set_decoration_layout(Some(":minimize,maximize,close"));
    header.set_show_title_buttons(true);
    header.set_title_widget(Some(&heading));

    let new_button = Button::from_icon_name("list-add-symbolic");
    new_button.set_tooltip_text(Some("New tab (Ctrl+Shift+T)"));
    new_button.add_css_class("veyl-header-button");
    header.pack_end(&new_button);

    let menu = MenuButton::new();
    menu.set_icon_name("pan-down-symbolic");
    menu.set_tooltip_text(Some("Appearance"));
    menu.add_css_class("veyl-header-button");
    header.pack_end(&menu);

    let stack = Stack::new();
    stack.set_hexpand(true);
    stack.set_vexpand(true);
    stack.add_css_class("veyl-stack");
    window.set_titlebar(Some(&header));
    window.set_child(Some(&stack));

    let state = Rc::new(RefCell::new(State {
        window: window.clone(),
        tabs_bar,
        stack,
        tabs: Vec::new(),
        next_id: 0,
        active: None,
        config,
        appearance,
    }));

    update_appearance(&state);
    build_appearance_menu(&menu, &state);

    {
        let state = state.clone();
        new_button.connect_clicked(move |_| open_tab(&state));
    }

    install_actions(&window, app, &state);
    open_tab(&state);
    window.present();
}

fn update_appearance(state: &Rc<RefCell<State>>) {
    let current = state.borrow();
    let color = gdk::RGBA::parse(&current.config.background)
        .unwrap_or_else(|_| gdk::RGBA::new(0.063, 0.071, 0.102, 1.0));
    let opacity = current.config.opacity;
    let css = format!(
        "window.veyl-window {{ background: transparent; }} \
         .veyl-stack, .veyl-terminal-surface {{ \
         background-color: rgba({},{},{},{:.3}); }}",
        (color.red() * 255.0).round() as u8,
        (color.green() * 255.0).round() as u8,
        (color.blue() * 255.0).round() as u8,
        opacity,
    );
    current.appearance.load_from_data(&css);
    for tab in &current.tabs {
        apply_terminal_settings(&tab.terminal, &current.config);
    }
}

fn apply_terminal_settings(terminal: &Terminal, config: &Config) {
    let fallback = gdk::RGBA::new(0.91, 0.92, 0.94, 1.0);
    let foreground = gdk::RGBA::parse(&config.foreground).unwrap_or(fallback);
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
    let body = GtkBox::new(Orientation::Vertical, 12);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(18);
    body.set_margin_end(18);
    body.set_size_request(230, -1);

    let title = Label::new(Some("Background opacity"));
    title.set_halign(gtk::Align::Start);
    let help = Label::new(Some("Blur depends on GNOME compositor support."));
    help.set_wrap(true);
    help.add_css_class("veyl-hint");
    help.set_halign(gtk::Align::Start);

    let slider = Scale::with_range(Orientation::Horizontal, 0.4, 1.0, 0.05);
    slider.set_draw_value(true);
    slider.set_digits(2);
    slider.set_value(state.borrow().config.opacity);
    {
        let state = state.clone();
        slider.connect_value_changed(move |scale| {
            {
                let mut ui = state.borrow_mut();
                ui.config.opacity = scale.value();
                ui.config.normalize();
                if let Err(err) = ui.config.save() {
                    eprintln!("Veyl: unable to save settings: {err}");
                }
            }
            update_appearance(&state);
        });
    }

    body.append(&title);
    body.append(&slider);
    body.append(&help);
    popover.set_child(Some(&body));
    menu.set_popover(Some(&popover));
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

    let select = Button::with_label(&format!("Terminal {id}"));
    select.add_css_class("veyl-tab-select");
    select.set_tooltip_text(Some("Activate tab"));

    let close = Button::from_icon_name("window-close-symbolic");
    close.add_css_class("veyl-tab-close");
    close.set_tooltip_text(Some("Close tab"));

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

    let shell = shell_path();
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
    if !ui.tabs.iter().any(|tab| tab.id == id) {
        return;
    }
    ui.active = Some(id);
    ui.stack.set_visible_child_name(&format!("tab-{id}"));
    for tab in &ui.tabs {
        if tab.id == id {
            tab.chip.add_css_class("active");
            tab.select.add_css_class("active");
        } else {
            tab.chip.remove_css_class("active");
            tab.select.remove_css_class("active");
        }
    }
    let focused = ui.tabs.iter().find(|tab| tab.id == id).map(|tab| tab.terminal.clone());
    drop(ui);
    if let Some(terminal) = focused {
        terminal.grab_focus();
    }
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
