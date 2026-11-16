mod palette;
mod panes;
mod search;
mod settings;
mod shortcuts;
mod tabs;
mod themes;

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
    Orientation, PolicyType, Popover, ScrolledWindow, Stack, Viewport,
};
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use vte4::prelude::*;
use vte4::Terminal;

struct Tab {
    id: u64,
    panes: Vec<panes::Pane>,
    active_pane: u64,
    surface: GtkBox,
    chip: GtkBox,
    select: Button,
    close: Button,
    label: Label,
    default_title: String,
    color: Option<gdk::RGBA>,
    menu: gtk::PopoverMenu,
    progress: gtk::ProgressBar,
    progress_timer: Option<glib::SourceId>,
}

impl Tab {
    fn active_pane(&self) -> &panes::Pane {
        self.panes
            .iter()
            .find(|pane| pane.id == self.active_pane)
            .expect("a tab owns its active pane")
    }
}

struct SettingsTab {
    workspace: settings::Workspace,
    chip: GtkBox,
    select: Button,
    return_to: Option<u64>,
}

struct State {
    window: glib::WeakRef<ApplicationWindow>,
    tabs_bar: GtkBox,
    tabs_viewport: Viewport,
    stack: Stack,
    tabs: Vec<Tab>,
    settings: Option<SettingsTab>,
    next_id: u64,
    next_pane: u64,
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
    menu.set_tooltip_text(Some("Tab menu"));
    menu.update_property(&[Property::Label("Tab menu")]);
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
        settings: None,
        next_id: 0,
        next_pane: 0,
        active: None,
        closing: false,
        config,
        appearance: appearance.clone(),
        appearance_class,
        blur_supported,
    }));

    update_appearance(&state);
    build_tab_menu(&menu, &state);

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
    let mut css = format!(
        "window.{class} {{ background-color: rgba({},{},{},{:.3}); {backdrop_filter} }} \
         window.{class} .veyl-terminal-surface {{ padding: {}px {}px; }}",
        (color.red() * 255.0).round() as u8,
        (color.green() * 255.0).round() as u8,
        (color.blue() * 255.0).round() as u8,
        opacity,
        current.config.padding_y,
        current.config.padding_x,
    );
    tabs::append_color_rules(&mut css, &current);
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
    let foreground = appearance_color(&config.foreground, "#e8eaf0");
    let scheme = themes::resolve(config, &config.color_scheme);
    let palette = scheme
        .as_ref()
        .and_then(themes::palette)
        .unwrap_or_default();
    let palette = palette.iter().collect::<Vec<_>>();
    terminal.set_colors(Some(&foreground), None, &palette);
    let cursor = scheme
        .as_ref()
        .and_then(|scheme| scheme.cursor.as_deref())
        .and_then(|color| gdk::RGBA::parse(color).ok());
    let selection = scheme
        .as_ref()
        .and_then(|scheme| scheme.selection.as_deref())
        .and_then(|color| gdk::RGBA::parse(color).ok());
    terminal.set_color_cursor(cursor.as_ref());
    terminal.set_color_highlight(selection.as_ref());
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

fn open_settings(state: &Rc<RefCell<State>>, page: &str) {
    let (config, blur_supported, tabs_bar, stack) = {
        let ui = state.borrow();
        (
            ui.config.clone(),
            ui.blur_supported,
            ui.tabs_bar.clone(),
            ui.stack.clone(),
        )
    };
    if state.borrow().settings.is_none() {
        let weak_state = Rc::downgrade(state);
        let workspace = settings::build(
            &config,
            blur_supported,
            move |draft| {
                if let Some(state) = weak_state.upgrade() {
                    save_settings(&state, draft)?;
                }
                Ok(())
            },
            {
                let weak_state = Rc::downgrade(state);
                move || {
                    if let Some(state) = weak_state.upgrade() {
                        close_settings(&state);
                    }
                }
            },
        );
        let chip = GtkBox::new(Orientation::Horizontal, 0);
        chip.add_css_class("veyl-tab");
        let select = menu_row("Settings", "emblem-system-symbolic");
        select.add_css_class("veyl-tab-select");
        select.connect_clicked(glib::clone!(
            #[weak]
            state,
            move |_| activate_settings(&state)
        ));
        let close = Button::from_icon_name("window-close-symbolic");
        close.add_css_class("veyl-tab-close");
        close.set_tooltip_text(Some("Close Settings"));
        close.update_property(&[Property::Label("Close Settings")]);
        close.connect_clicked(glib::clone!(
            #[weak]
            state,
            move |_| close_settings(&state)
        ));
        chip.append(&select);
        chip.append(&close);
        tabs_bar.append(&chip);
        stack.add_child(&workspace.surface);
        state.borrow_mut().settings = Some(SettingsTab {
            workspace,
            chip,
            select,
            return_to: None,
        });
    }
    let pages = state
        .borrow()
        .settings
        .as_ref()
        .unwrap()
        .workspace
        .pages
        .clone();
    pages.set_visible_child_name(page);
    activate_settings(state);
}

fn activate_settings(state: &Rc<RefCell<State>>) {
    let (stack, surface, chip, select, previous, viewport) = {
        let mut ui = state.borrow_mut();
        let active = ui.active.take();
        let previous = ui
            .tabs
            .iter()
            .find(|tab| Some(tab.id) == active)
            .map(|tab| (tab.chip.clone(), tab.select.clone()));
        let settings = ui.settings.as_mut().expect("Settings tab is open");
        if active.is_some() {
            settings.return_to = active;
        }
        let widgets = (
            settings.workspace.surface.clone(),
            settings.chip.clone(),
            settings.select.clone(),
        );
        (
            ui.stack.clone(),
            widgets.0,
            widgets.1,
            widgets.2,
            previous,
            ui.tabs_viewport.clone(),
        )
    };
    if let Some((chip, select)) = previous {
        chip.remove_css_class("active");
        select.update_state(&[AccessibleState::Pressed(gtk::AccessibleTristate::False)]);
    }
    stack.set_visible_child(&surface);
    chip.add_css_class("active");
    select.update_state(&[AccessibleState::Pressed(gtk::AccessibleTristate::True)]);
    select.grab_focus();
    viewport.add_tick_callback(glib::clone!(
        #[weak]
        chip,
        #[upgrade_or]
        glib::ControlFlow::Break,
        move |viewport, _| {
            if !chip.has_css_class("active") {
                return glib::ControlFlow::Break;
            }
            if chip.width() == 0 {
                return glib::ControlFlow::Continue;
            }
            viewport.scroll_to(&chip, None);
            glib::ControlFlow::Break
        }
    ));
    panes::update_focus_actions(state);
}

fn close_settings(state: &Rc<RefCell<State>>) {
    let (settings, next, window, tabs_bar, stack) = {
        let mut ui = state.borrow_mut();
        let Some(settings) = ui.settings.take() else {
            return;
        };
        let next = ui
            .tabs
            .iter()
            .find(|tab| Some(tab.id) == settings.return_to)
            .or_else(|| ui.tabs.last())
            .map(|tab| tab.id);
        (
            settings,
            next,
            ui.window.upgrade(),
            ui.tabs_bar.clone(),
            ui.stack.clone(),
        )
    };
    let was_active = settings.chip.has_css_class("active");
    tabs_bar.remove(&settings.chip);
    stack.remove(&settings.workspace.surface);
    if was_active {
        if let Some(id) = next {
            activate_tab(state, id);
        } else if let Some(window) = window {
            window.close();
        }
    }
}

fn save_settings(state: &Rc<RefCell<State>>, draft: Config) -> std::io::Result<()> {
    let (original, window) = {
        let ui = state.borrow();
        (ui.config.clone(), ui.window.upgrade())
    };
    draft.save_changes(&original)?;
    if let Some(app) = window.and_then(|window| window.application()) {
        if original.shortcuts != draft.shortcuts {
            shortcuts::apply(&app, &draft);
        }
    }
    let (terminals, ids) = {
        let mut ui = state.borrow_mut();
        ui.config = draft.clone();
        let terminals = ui
            .tabs
            .iter()
            .flat_map(|tab| &tab.panes)
            .map(|pane| (pane.terminal.clone(), pane.appearance(&draft)))
            .collect::<Vec<_>>();
        let ids = ui.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>();
        (terminals, ids)
    };
    update_appearance(state);
    for (terminal, settings) in terminals {
        apply_terminal_appearance(&terminal, &settings);
    }
    for id in ids {
        panes::refresh_progress(state, id);
    }
    Ok(())
}

fn menu_row(text: &str, icon: &str) -> Button {
    let row = GtkBox::new(Orientation::Horizontal, 10);
    row.append(&Image::from_icon_name(icon));
    let label = Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_max_width_chars(27);
    label.set_ellipsize(pango::EllipsizeMode::End);
    row.append(&label);
    let button = Button::new();
    button.set_child(Some(&row));
    button.set_tooltip_text(Some(text));
    button.update_property(&[Property::Label(text)]);
    button
}

fn build_tab_menu(menu: &MenuButton, state: &Rc<RefCell<State>>) {
    let popover = Popover::new();
    popover.add_css_class("veyl-launch-menu");
    popover.set_has_arrow(false);
    popover.set_halign(gtk::Align::Start);
    let body = GtkBox::new(Orientation::Vertical, 2);

    let config = state.borrow().config.clone();
    for profile in config.profiles.keys() {
        let button = menu_row(profile, "utilities-terminal-symbolic");
        button.set_action_name(Some("win.open-profile"));
        button.set_action_target_value(Some(&profile.to_variant()));
        button.connect_clicked(glib::clone!(
            #[weak]
            popover,
            move |_| popover.popdown()
        ));
        body.append(&button);
    }
    if !config.profiles.is_empty() {
        body.append(&gtk::Separator::new(Orientation::Horizontal));
    }

    for (label, icon, action) in [
        ("Split right", "view-dual-symbolic", "win.split-right"),
        (
            "Split down",
            "view-split-left-right-symbolic",
            "win.split-down",
        ),
        ("Find in terminal", "edit-find-symbolic", "win.find"),
    ] {
        let button = menu_row(label, icon);
        button.set_action_name(Some(action));
        button.connect_clicked(glib::clone!(
            #[weak]
            popover,
            move |_| popover.popdown()
        ));
        body.append(&button);
    }
    body.append(&gtk::Separator::new(Orientation::Horizontal));
    for (label, icon, page) in [
        ("Settings…", "emblem-system-symbolic", "appearance"),
        (
            "Keyboard shortcuts…",
            "input-keyboard-symbolic",
            "shortcuts",
        ),
    ] {
        let button = menu_row(label, icon);
        button.connect_clicked(glib::clone!(
            #[weak]
            state,
            #[weak]
            popover,
            move |_| {
                popover.popdown();
                open_settings(&state, page);
            }
        ));
        body.append(&button);
    }

    let palette = menu_row("Command Palette…", "system-search-symbolic");
    palette.set_action_name(Some("win.command-palette"));
    palette.connect_clicked(glib::clone!(
        #[weak]
        popover,
        move |_| popover.popdown()
    ));
    body.append(&palette);

    let scrolled = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::Never)
        .vscrollbar_policy(PolicyType::Automatic)
        .propagate_natural_width(true)
        .min_content_width(230)
        .max_content_height(360)
        .propagate_natural_height(true)
        .child(&body)
        .build();
    popover.set_child(Some(&scrolled));
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
    let launch = {
        let ui = state.borrow();
        panes::Launch::for_profile(&ui.config, &ui.config.default_profile)
    };
    open_tab_with_launch(state, launch);
}

#[cfg(test)]
fn open_tab_with_shell(state: &Rc<RefCell<State>>, shell: &str, home: Option<&str>) {
    open_tab_with_launch(
        state,
        panes::Launch {
            shell: shell.into(),
            directory: home.map(str::to_owned),
            profile: None,
        },
    );
}

fn open_tab_with_launch(state: &Rc<RefCell<State>>, launch: panes::Launch) {
    let (id, title, tabs_bar, stack) = {
        let mut ui = state.borrow_mut();
        if ui.closing {
            return;
        }
        ui.next_id += 1;
        (
            ui.next_id,
            launch.title(&ui.config),
            ui.tabs_bar.clone(),
            ui.stack.clone(),
        )
    };
    let pane = panes::Pane::new(state, launch);
    let pane_id = pane.id;
    let surface = GtkBox::new(Orientation::Vertical, 0);
    surface.add_css_class("veyl-terminal-surface");
    surface.append(&pane.search.overlay);

    let chip = GtkBox::new(Orientation::Horizontal, 0);
    chip.add_css_class("veyl-tab");
    chip.add_css_class(&format!("veyl-tab-{id}"));

    let label = Label::new(Some(&title));
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
    select.set_tooltip_text(Some(&title));
    select.update_property(&[Property::Label(&format!("Tab {id}, {title}"))]);

    let close = Button::from_icon_name("window-close-symbolic");
    close.add_css_class("veyl-tab-close");
    close.set_tooltip_text(Some("Close tab"));
    close.update_property(&[Property::Label(&format!("Close tab {id}, {title}"))]);

    let row = GtkBox::new(Orientation::Horizontal, 0);
    row.append(&select);
    row.append(&close);
    let overlay = gtk::Overlay::new();
    overlay.set_child(Some(&row));
    let progress = gtk::ProgressBar::new();
    progress.set_valign(gtk::Align::End);
    progress.add_css_class("veyl-tab-progress");
    progress.set_can_target(false);
    progress.set_visible(false);
    progress.update_property(&[Property::Label("Terminal progress")]);
    overlay.add_overlay(&progress);
    chip.append(&overlay);

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

    let menu = tabs::context_menu(&select, id);
    tabs::enable_reorder(state, &chip, &select, id);
    state.borrow_mut().tabs.push(Tab {
        id,
        panes: vec![pane],
        active_pane: pane_id,
        surface: surface.clone(),
        chip: chip.clone(),
        select,
        close,
        label,
        default_title: title,
        color: None,
        menu,
        progress,
        progress_timer: None,
    });
    stack.add_child(&surface);
    tabs_bar.append(&chip);
    if let Some(settings) = state.borrow().settings.as_ref() {
        tabs_bar.reorder_child_after(&settings.chip, Some(&chip));
    }
    activate_tab(state, id);
    panes::spawn(state, id, pane_id);
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
            tab.active_pane().terminal.clone(),
            previous
                .map(|tab| (tab.chip.clone(), tab.select.clone()))
                .or_else(|| {
                    ui.settings
                        .as_ref()
                        .map(|tab| (tab.chip.clone(), tab.select.clone()))
                }),
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
    let weak_state = Rc::downgrade(state);
    viewport.add_tick_callback(move |viewport, _| {
        let Some(chip) = chip.upgrade() else {
            return glib::ControlFlow::Break;
        };
        if !weak_state
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
    panes::update_focus_actions(state);
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
        let window = if ui.tabs.is_empty() && ui.settings.is_none() {
            ui.closing = true;
            ui.window.upgrade()
        } else {
            None
        };
        (tab, next, window, ui.tabs_bar.clone(), ui.stack.clone())
    };
    let had_color = tab.color.is_some();
    detach_tab(tab, &tabs_bar, &stack);
    if had_color {
        update_appearance(state);
    }
    if let Some(window) = window {
        window.close();
    } else if let Some(id) = next {
        activate_tab(state, id);
    } else if state.borrow().active.is_none()
        && state.borrow().tabs.is_empty()
        && state.borrow().settings.is_some()
    {
        activate_settings(state);
    }
}

fn close_all_tabs(state: &Rc<RefCell<State>>) {
    let (tabs, settings, tabs_bar, stack) = {
        let mut ui = state.borrow_mut();
        ui.closing = true;
        ui.active = None;
        (
            std::mem::take(&mut ui.tabs),
            ui.settings.take(),
            ui.tabs_bar.clone(),
            ui.stack.clone(),
        )
    };
    if let Some(settings) = settings {
        tabs_bar.remove(&settings.chip);
        stack.remove(&settings.workspace.surface);
    }
    for tab in tabs {
        detach_tab(tab, &tabs_bar, &stack);
    }
}

fn detach_tab(tab: Tab, tabs_bar: &GtkBox, stack: &Stack) {
    if let Some(timer) = tab.progress_timer {
        timer.remove();
    }
    for pane in &tab.panes {
        pane.spawn.cancel();
    }
    tab.menu.popdown();
    tab.menu.unparent();
    // Let VTE dispose the terminal, signal its child, and release the PTY.
    tabs_bar.remove(&tab.chip);
    stack.remove(&tab.surface);
}

fn active_terminal(state: &Rc<RefCell<State>>) -> Option<Terminal> {
    let ui = state.borrow();
    ui.tabs
        .iter()
        .find(|tab| Some(tab.id) == ui.active)
        .map(|tab| tab.active_pane().terminal.clone())
}

fn install_actions(window: &ApplicationWindow, app: &Application, state: &Rc<RefCell<State>>) {
    let settings = gio::SimpleAction::new("settings", None);
    settings.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            open_settings(&state, "appearance");
        }
    ));
    window.add_action(&settings);
    let palette = gio::SimpleAction::new("command-palette", None);
    palette.connect_activate(glib::clone!(
        #[weak]
        window,
        #[weak]
        state,
        move |_, _| {
            let profiles = state.borrow().config.profiles.clone();
            palette::show(&window, &profiles);
        }
    ));
    window.add_action(&palette);

    let new_tab = gio::SimpleAction::new("new-tab", None);
    new_tab.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| open_tab(&state)
    ));
    window.add_action(&new_tab);

    let profile = gio::SimpleAction::new("open-profile", Some(&String::static_variant_type()));
    profile.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, target| {
            let Some(name) = target.and_then(glib::Variant::str) else {
                return;
            };
            let launch = {
                let ui = state.borrow();
                ui.config
                    .profiles
                    .contains_key(name)
                    .then(|| panes::Launch::for_profile(&ui.config, name))
            };
            if let Some(launch) = launch {
                open_tab_with_launch(&state, launch);
            }
        }
    ));
    window.add_action(&profile);

    let close = gio::SimpleAction::new("close-tab", None);
    close.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            if state.borrow().active.is_none() {
                close_settings(&state);
            } else {
                panes::close_active(&state);
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

    let find = gio::SimpleAction::new("find", None);
    find.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            let active = {
                let ui = state.borrow();
                ui.tabs
                    .iter()
                    .find(|tab| Some(tab.id) == ui.active)
                    .map(|tab| {
                        (
                            tab.active_pane().search.clone(),
                            tab.active_pane().terminal.clone(),
                        )
                    })
            };
            if let Some((search, terminal)) = active {
                search.show(&terminal);
            }
        }
    ));
    window.add_action(&find);

    panes::install_actions(window, state);
    tabs::install_actions(window, state);
    shortcuts::apply(app, &state.borrow().config);
}

#[cfg(test)]
mod tests;
