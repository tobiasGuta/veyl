use super::{apply_terminal_settings, close_tab, search::Search, shell_path, State};
use crate::config::Config;
use gtk::accessible::Property;
use gtk::prelude::*;
use gtk::{gio, glib, ApplicationWindow, Orientation, Paned, Widget};
use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use vte4::{prelude::*, PtyFlags, Terminal};

const PROGRESS_VALUE: &str = "vte.progress.value";
const PROGRESS_HINT: &str = "vte.progress.hint";

#[derive(Clone)]
pub(super) struct Launch {
    pub(super) shell: String,
    pub(super) directory: Option<String>,
    pub(super) profile: Option<String>,
}

impl Launch {
    pub(super) fn for_profile(config: &Config, name: &str) -> Self {
        let profile = config.profiles.get(name);
        let home = std::env::var("HOME").ok();
        let directory = profile
            .and_then(|profile| profile.directory.as_deref())
            .and_then(|path| {
                if path.is_absolute() {
                    Some(path.to_path_buf())
                } else {
                    path.strip_prefix("~")
                        .ok()
                        .and_then(|path| home.as_ref().map(|home| Path::new(home).join(path)))
                }
            });
        let shell = profile
            .and_then(|profile| profile.shell.as_ref())
            .filter(|shell| Path::new(shell).is_absolute())
            .cloned();
        Self {
            shell: shell.unwrap_or_else(shell_path),
            directory: directory
                .and_then(|path| path.into_os_string().into_string().ok())
                .or(home),
            profile: profile.map(|_| name.to_owned()),
        }
    }

    pub(super) fn title(&self, config: &Config) -> String {
        if let Some(name) = &self.profile {
            return config
                .profiles
                .get(name)
                .and_then(|profile| profile.tab_name.clone())
                .filter(|title| !title.is_empty())
                .unwrap_or_else(|| name.clone());
        }
        Path::new(&self.shell)
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("Terminal")
            .into()
    }

    fn settings(&self, config: &Config) -> Config {
        let mut settings = config.clone();
        if let Some(scheme) = self
            .profile
            .as_ref()
            .and_then(|name| config.profiles.get(name))
            .and_then(|profile| profile.color_scheme.as_ref())
        {
            settings.color_scheme = scheme.clone();
        }
        settings
    }

    pub(super) fn duplicate(&self, terminal: &Terminal, pid: Option<glib::Pid>) -> Self {
        let directory = current_directory(terminal, pid)
            .and_then(|path| path.into_os_string().into_string().ok());
        Self {
            directory: directory.or_else(|| self.directory.clone()),
            ..self.clone()
        }
    }
}

pub(super) fn current_directory(terminal: &Terminal, pid: Option<glib::Pid>) -> Option<PathBuf> {
    // gtk-rs does not yet wrap the URI termprop getter.
    let directory = match terminal.property::<Option<glib::GString>>("current-directory-uri") {
        Some(uri) => local_directory(&uri),
        None => pid.and_then(|pid| std::fs::read_link(format!("/proc/{}/cwd", pid.0)).ok()),
    };
    directory.filter(|path| path.is_dir())
}

fn local_directory(uri: &str) -> Option<PathBuf> {
    let (path, host) = glib::filename_from_uri(uri).ok()?;
    if let Some(host) = host {
        if !host.is_empty()
            && !host.eq_ignore_ascii_case("localhost")
            && !host.eq_ignore_ascii_case(&glib::host_name())
        {
            return None;
        }
    }
    Some(path)
}

pub(super) struct Pane {
    pub(super) id: u64,
    pub(super) terminal: Terminal,
    pub(super) search: Search,
    pub(super) launch: Launch,
    pub(super) spawn: gio::Cancellable,
    pub(super) pid: Option<glib::Pid>,
}

impl Pane {
    pub(super) fn new(state: &Rc<RefCell<State>>, launch: Launch) -> Self {
        let (id, settings) = {
            let mut ui = state.borrow_mut();
            ui.next_pane += 1;
            (ui.next_pane, launch.settings(&ui.config))
        };
        let terminal = Terminal::new();
        terminal.set_hexpand(true);
        terminal.set_vexpand(true);
        terminal.add_css_class("veyl-terminal");
        apply_terminal_settings(&terminal, &settings);
        let search = Search::new(&terminal);
        search.overlay.add_css_class("veyl-terminal-surface");
        super::links::install(&terminal, state);
        Self {
            id,
            terminal,
            search,
            launch,
            spawn: gio::Cancellable::new(),
            pid: None,
        }
    }

    pub(super) fn appearance(&self, config: &Config) -> Config {
        self.launch.settings(config)
    }
}

pub(super) fn spawn(state: &Rc<RefCell<State>>, tab_id: u64, pane_id: u64) {
    let (terminal, launch, cancel, window, prompt) = {
        let ui = state.borrow();
        let pane = ui
            .tabs
            .iter()
            .find(|tab| tab.id == tab_id)
            .and_then(|tab| tab.panes.iter().find(|pane| pane.id == pane_id))
            .expect("attach the pane before spawning");
        (
            pane.terminal.clone(),
            pane.launch.clone(),
            pane.spawn.clone(),
            ui.window.upgrade(),
            ui.config.bash_prompt.clone(),
        )
    };
    terminal.connect_child_exited(glib::clone!(
        #[weak]
        state,
        move |_, _| {
            close_pane(&state, tab_id, pane_id);
        }
    ));
    terminal.connect_has_focus_notify(glib::clone!(
        #[weak]
        state,
        move |terminal| {
            if terminal.has_focus() {
                set_active_pane(&state, tab_id, pane_id);
            }
        }
    ));
    terminal.connect_termprop_changed(
        Some("vte.cwd"),
        glib::clone!(
            #[weak]
            state,
            move |_, _| super::tabs::refresh_title(&state, tab_id)
        ),
    );
    for property in [PROGRESS_VALUE, PROGRESS_HINT] {
        terminal.connect_termprop_changed(
            Some(property),
            glib::clone!(
                #[weak]
                state,
                move |_, _| {
                    refresh_progress(&state, tab_id);
                }
            ),
        );
    }
    let hold = window
        .and_then(|window| window.application())
        .map(|app| app.hold());
    let shell_for_error = launch.shell.clone();
    let weak = Rc::downgrade(state);
    let mut argv = vec![launch.shell.as_str()];
    let mut environment = Vec::new();
    let startup;
    let prompt_variable;
    if let Some(prompt) =
        prompt.filter(|value| !value.is_empty() && Path::new(&launch.shell).ends_with("bash"))
    {
        let prepared = crate::prompt::validate(&prompt).and_then(|()| {
            crate::prompt::startup_file()?
                .into_os_string()
                .into_string()
                .map_err(|_| std::io::Error::other("the prompt startup path is not UTF-8"))
        });
        match prepared {
            Ok(path) => {
                startup = path;
                prompt_variable = format!("VEYL_BASH_PROMPT={prompt}");
                argv.extend(["--rcfile", &startup]);
                environment.push(prompt_variable.as_str());
            }
            Err(err) => {
                let detail = err.to_string().replace(char::is_control, " ");
                terminal.feed(
                    format!("Veyl could not load the custom Bash prompt. {detail}\r\n").as_bytes(),
                );
            }
        }
    }
    terminal.spawn_async(
        PtyFlags::DEFAULT,
        launch.directory.as_deref(),
        &argv,
        &environment,
        glib::SpawnFlags::DEFAULT,
        || {},
        -1,
        Some(&cancel),
        move |result| {
            let _hold = hold;
            let Some(state) = weak.upgrade() else { return };
            let terminal = {
                let mut ui = state.borrow_mut();
                let Some(pane) = ui.tabs.iter_mut().find(|tab| tab.id == tab_id)
                    .and_then(|tab| tab.panes.iter_mut().find(|pane| pane.id == pane_id)) else { return };
                match result {
                    Ok(pid) => {
                        pane.pid = Some(pid);
                        return;
                    }
                    Err(_) => pane.terminal.clone(),
                }
            };
            if let Err(err) = result {
                let detail = err.message().replace(char::is_control, " ");
                let message = format!(
                    "Veyl could not start the shell {shell_for_error:?}.\r\n{detail}\r\n\
                     Check that the shell is executable and the starting directory is accessible.\r\n"
                );
                terminal.set_input_enabled(false);
                terminal.feed(message.as_bytes());
            }
        },
    );
}

fn set_active_pane(state: &Rc<RefCell<State>>, tab_id: u64, pane_id: u64) {
    {
        let mut ui = state.borrow_mut();
        if ui.active != Some(tab_id) {
            return;
        }
        let Some(tab) = ui.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        if !tab.panes.iter().any(|pane| pane.id == pane_id) {
            return;
        }
        tab.active_pane = pane_id;
    }
    refresh_progress(state, tab_id);
    super::tabs::refresh_title(state, tab_id);
}

pub(super) fn refresh_progress(state: &Rc<RefCell<State>>, tab_id: u64) {
    let (bar, value, hint, old_timer) = {
        let mut ui = state.borrow_mut();
        let enabled = ui.config.tab_progress;
        let Some(tab) = ui.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return;
        };
        let terminal = &tab.active_pane().terminal;
        let value = enabled
            .then(|| terminal.termprop_uint(PROGRESS_VALUE))
            .flatten();
        let hint = terminal.termprop_int(PROGRESS_HINT);
        (tab.progress.clone(), value, hint, tab.progress_timer.take())
    };
    if let Some(timer) = old_timer {
        timer.remove();
    }
    bar.set_visible(value.is_some());
    bar.remove_css_class("error");
    bar.remove_css_class("paused");
    let Some(value) = value else { return };
    match hint.and_then(|hint| i32::try_from(hint).ok()) {
        Some(vte4::ffi::VTE_PROGRESS_HINT_INDETERMINATE) => {
            bar.pulse();
            let timer = glib::timeout_add_local(
                std::time::Duration::from_millis(120),
                glib::clone!(
                    #[weak]
                    bar,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || {
                        bar.pulse();
                        glib::ControlFlow::Continue
                    }
                ),
            );
            if let Some(tab) = state
                .borrow_mut()
                .tabs
                .iter_mut()
                .find(|tab| tab.id == tab_id)
            {
                tab.progress_timer = Some(timer);
            } else {
                timer.remove();
            }
        }
        hint => {
            if hint == Some(vte4::ffi::VTE_PROGRESS_HINT_ERROR) {
                bar.add_css_class("error");
            } else if hint == Some(vte4::ffi::VTE_PROGRESS_HINT_PAUSED) {
                bar.add_css_class("paused");
            }
            bar.set_fraction(value.min(100) as f64 / 100.0);
        }
    }
}

enum Slot {
    Root(gtk::Box),
    Start(Paned),
    End(Paned),
}

impl Slot {
    fn containing(child: &Widget) -> Option<Self> {
        let parent = child.parent()?;
        if let Some(paned) = parent.downcast_ref::<Paned>() {
            if paned.start_child().as_ref() == Some(child) {
                Some(Self::Start(paned.clone()))
            } else {
                Some(Self::End(paned.clone()))
            }
        } else {
            parent.downcast().ok().map(Self::Root)
        }
    }

    fn remove(&self, child: &Widget) {
        match self {
            Self::Root(root) => root.remove(child),
            Self::Start(split) => split.set_start_child(Widget::NONE),
            Self::End(split) => split.set_end_child(Widget::NONE),
        }
    }

    fn add(&self, child: &Widget) {
        match self {
            Self::Root(root) => root.append(child),
            Self::Start(split) => split.set_start_child(Some(child)),
            Self::End(split) => split.set_end_child(Some(child)),
        }
    }
}

fn unzoom(state: &Rc<RefCell<State>>, tab_id: u64) -> bool {
    let siblings = {
        let mut ui = state.borrow_mut();
        let Some(tab) = ui.tabs.iter_mut().find(|tab| tab.id == tab_id) else {
            return false;
        };
        std::mem::take(&mut tab.zoomed_siblings)
    };
    let was_zoomed = !siblings.is_empty();
    for (split, sibling, position) in siblings {
        sibling.set_visible(true);
        split.set_position(position);
    }
    was_zoomed
}

fn toggle_zoom(state: &Rc<RefCell<State>>) {
    let (tab_id, mut child) = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| Some(tab.id) == ui.active) else {
            return;
        };
        (
            tab.id,
            tab.active_pane().search.overlay.clone().upcast::<Widget>(),
        )
    };
    if !unzoom(state, tab_id) {
        let mut siblings = Vec::new();
        while let Some(split) = child.parent().and_downcast::<Paned>() {
            let sibling = if split.start_child().as_ref() == Some(&child) {
                split.end_child()
            } else {
                split.start_child()
            };
            if let Some(sibling) = sibling {
                siblings.push((split.clone(), sibling, split.position()));
            }
            child = split.upcast();
        }
        {
            let mut ui = state.borrow_mut();
            let tab = ui.tabs.iter_mut().find(|tab| tab.id == tab_id).unwrap();
            tab.zoomed_siblings = siblings.clone();
        }
        for (_, sibling, _) in siblings {
            sibling.set_visible(false);
        }
    }
    update_focus_actions(state);
}

pub(super) fn split_active(state: &Rc<RefCell<State>>, orientation: Orientation) {
    let (tab_id, overlay, launch, select) = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| Some(tab.id) == ui.active) else {
            return;
        };
        let pane = tab.active_pane();
        (
            tab.id,
            pane.search.overlay.clone(),
            pane.launch.duplicate(&pane.terminal, pane.pid),
            tab.select.clone(),
        )
    };
    unzoom(state, tab_id);
    let current = overlay.upcast::<Widget>();
    let Some(slot) = Slot::containing(&current) else {
        return;
    };
    let pane = Pane::new(state, launch);
    let new_overlay = pane.search.overlay.clone();
    let terminal = pane.terminal.clone();
    let pane_id = pane.id;
    {
        let mut ui = state.borrow_mut();
        let tab = ui.tabs.iter_mut().find(|tab| tab.id == tab_id).unwrap();
        tab.active_pane = pane_id;
        tab.panes.push(pane);
    }
    let extent = if orientation == Orientation::Horizontal {
        current.width()
    } else {
        current.height()
    };
    let split = Paned::new(orientation);
    split.set_resize_start_child(true);
    split.set_resize_end_child(true);
    if extent > 0 {
        split.set_position(extent / 2);
    }
    split.add_css_class("veyl-split");
    split.update_property(&[Property::Label("Terminal split")]);
    split.set_shrink_start_child(false);
    split.set_shrink_end_child(false);
    // Move focus outside the subtree before GTK reparents it.
    select.grab_focus();
    slot.remove(&current);
    split.set_start_child(Some(&current));
    split.set_end_child(Some(&new_overlay));
    slot.add(split.upcast_ref());
    terminal.grab_focus();
    spawn(state, tab_id, pane_id);
    update_focus_actions(state);
    refresh_progress(state, tab_id);
}

pub(super) fn close_active(state: &Rc<RefCell<State>>) {
    let active = {
        let ui = state.borrow();
        ui.tabs
            .iter()
            .find(|tab| Some(tab.id) == ui.active)
            .map(|tab| (tab.id, tab.active_pane))
    };
    if let Some((tab_id, pane_id)) = active {
        close_pane(state, tab_id, pane_id);
    }
}

fn close_pane(state: &Rc<RefCell<State>>, tab_id: u64, pane_id: u64) {
    let (overlay, only_pane) = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| tab.id == tab_id) else {
            return;
        };
        let Some(pane) = tab.panes.iter().find(|pane| pane.id == pane_id) else {
            return;
        };
        (pane.search.overlay.clone(), tab.panes.len() == 1)
    };
    if only_pane {
        close_tab(state, tab_id);
        return;
    }
    unzoom(state, tab_id);
    let child = overlay.upcast::<Widget>();
    let Some(split) = child.parent().and_downcast::<Paned>() else {
        return;
    };
    let Some(parent) = Slot::containing(split.upcast_ref()) else {
        return;
    };
    let was_start = split.start_child().as_ref() == Some(&child);
    let sibling = if was_start {
        split.end_child()
    } else {
        split.start_child()
    };
    let Some(sibling) = sibling else { return };
    let (pane, select, focus) = {
        let mut ui = state.borrow_mut();
        let show_focus = ui.active == Some(tab_id);
        let focused = ui
            .window
            .upgrade()
            .as_ref()
            .and_then(gtk::prelude::GtkWindowExt::focus)
            .filter(|focus| focus.is_ancestor(&split));
        let tab = ui.tabs.iter_mut().find(|tab| tab.id == tab_id).unwrap();
        let index = tab
            .panes
            .iter()
            .position(|pane| pane.id == pane_id)
            .unwrap();
        let pane = tab.panes.remove(index);
        if tab.active_pane == pane_id {
            let next = tab
                .panes
                .iter()
                .find(|pane| {
                    let overlay = pane.search.overlay.upcast_ref::<Widget>();
                    *overlay == sibling || overlay.is_ancestor(&sibling)
                })
                .unwrap_or(&tab.panes[0]);
            tab.active_pane = next.id;
        }
        let select = (show_focus && focused.is_some()).then(|| tab.select.clone());
        let focus = focused.map(|focus| {
            if focus.is_ancestor(&sibling) {
                focus
            } else {
                tab.active_pane().terminal.clone().upcast()
            }
        });
        (pane, select, focus)
    };
    if let Some(select) = select {
        select.grab_focus();
    }
    pane.spawn.cancel();
    split.set_start_child(Widget::NONE);
    split.set_end_child(Widget::NONE);
    parent.remove(split.upcast_ref());
    parent.add(&sibling);
    if let Some(focus) = focus {
        focus.grab_focus();
    }
    update_focus_actions(state);
    refresh_progress(state, tab_id);
    super::tabs::refresh_title(state, tab_id);
}

fn focus_neighbor(state: &Rc<RefCell<State>>, dx: f32, dy: f32) {
    let target = {
        let ui = state.borrow();
        let Some(tab) = ui.tabs.iter().find(|tab| Some(tab.id) == ui.active) else {
            return;
        };
        let Some(current) = tab.active_pane().terminal.compute_bounds(&tab.surface) else {
            return;
        };
        let center = |rect: gtk::graphene::Rect| {
            (
                rect.x() + rect.width() / 2.0,
                rect.y() + rect.height() / 2.0,
            )
        };
        let (x, y) = center(current);
        tab.panes
            .iter()
            .filter(|pane| pane.id != tab.active_pane)
            .filter_map(|pane| {
                let (next_x, next_y) = center(pane.terminal.compute_bounds(&tab.surface)?);
                let forward = (next_x - x) * dx + (next_y - y) * dy;
                let side = ((next_x - x) * dy + (next_y - y) * dx).abs();
                (forward > 0.0).then(|| (forward + side * 2.0, pane.terminal.clone()))
            })
            .min_by(|first, second| first.0.total_cmp(&second.0))
            .map(|(_, terminal)| terminal)
    };
    if let Some(terminal) = target {
        terminal.grab_focus();
    }
}

pub(super) fn update_focus_actions(state: &Rc<RefCell<State>>) {
    let (window, split, zoomed) = {
        let ui = state.borrow();
        let tab = ui.tabs.iter().find(|tab| Some(tab.id) == ui.active);
        (
            ui.window.upgrade(),
            tab.is_some_and(|tab| tab.panes.len() > 1),
            tab.is_some_and(|tab| !tab.zoomed_siblings.is_empty()),
        )
    };
    if let Some(window) = window {
        if let Some(action) = window
            .lookup_action("zoom-pane")
            .and_downcast::<gio::SimpleAction>()
        {
            action.set_enabled(split);
        }
        for name in [
            "focus-pane-left",
            "focus-pane-right",
            "focus-pane-up",
            "focus-pane-down",
        ] {
            if let Some(action) = window
                .lookup_action(name)
                .and_downcast::<gio::SimpleAction>()
            {
                action.set_enabled(split && !zoomed);
            }
        }
    }
}

pub(super) fn install_actions(window: &ApplicationWindow, state: &Rc<RefCell<State>>) {
    let zoom = gio::SimpleAction::new("zoom-pane", None);
    zoom.set_enabled(false);
    zoom.connect_activate(glib::clone!(
        #[weak]
        state,
        move |_, _| toggle_zoom(&state)
    ));
    window.add_action(&zoom);
    for (name, orientation) in [
        ("split-right", Orientation::Horizontal),
        ("split-down", Orientation::Vertical),
    ] {
        let action = gio::SimpleAction::new(name, None);
        action.connect_activate(glib::clone!(
            #[weak]
            state,
            move |_, _| split_active(&state, orientation)
        ));
        window.add_action(&action);
    }
    for (name, x, y) in [
        ("focus-pane-left", -1.0, 0.0),
        ("focus-pane-right", 1.0, 0.0),
        ("focus-pane-up", 0.0, -1.0),
        ("focus-pane-down", 0.0, 1.0),
    ] {
        let action = gio::SimpleAction::new(name, None);
        action.set_enabled(false);
        action.connect_activate(glib::clone!(
            #[weak]
            state,
            move |_, _| focus_neighbor(&state, x, y)
        ));
        window.add_action(&action);
    }
}
