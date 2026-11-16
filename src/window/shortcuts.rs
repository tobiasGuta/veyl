use crate::config::Config;
use gtk::accessible::Property;
use gtk::prelude::*;
use gtk::{gdk, glib};
use std::collections::{BTreeMap, HashMap};

pub(super) const ACTIONS: &[(&str, &str, &str)] = &[
    ("new-tab", "New tab", "<Primary><Shift>t"),
    ("close-tab", "Close active terminal", "<Primary><Shift>w"),
    ("copy", "Copy", "<Primary><Shift>c"),
    ("paste", "Paste", "<Primary><Shift>v"),
    ("find", "Find in terminal", "<Primary><Shift>f"),
    ("duplicate-tab", "Duplicate tab", "<Primary><Shift>d"),
    ("next-tab", "Next tab", "<Primary>Tab"),
    ("previous-tab", "Previous tab", "<Primary><Shift>Tab"),
    ("split-right", "Split right", "<Alt><Shift>plus"),
    ("split-down", "Split down", "<Alt><Shift>minus"),
    ("focus-pane-left", "Focus pane to the left", "<Alt>Left"),
    ("focus-pane-right", "Focus pane to the right", "<Alt>Right"),
    ("focus-pane-up", "Focus pane above", "<Alt>Up"),
    ("focus-pane-down", "Focus pane below", "<Alt>Down"),
    ("settings", "Settings", "<Primary>comma"),
    ("zoom-pane", "Toggle pane zoom", "<Primary><Shift>Return"),
    (
        "increase-font-size",
        "Increase terminal font size",
        "<Primary>equal",
    ),
    (
        "decrease-font-size",
        "Decrease terminal font size",
        "<Primary>minus",
    ),
    ("reset-font-size", "Reset terminal font size", "<Primary>0"),
    ("command-palette", "Command Palette", "<Primary><Shift>p"),
];

fn defaults() -> BTreeMap<String, Vec<String>> {
    ACTIONS
        .iter()
        .map(|(id, _, shortcut)| ((*id).into(), vec![(*shortcut).into()]))
        .collect()
}

fn effective(config: &Config) -> BTreeMap<String, Vec<String>> {
    let mut bindings = defaults();
    for (id, keys) in &config.shortcuts {
        if let Some(binding) = bindings.get_mut(id) {
            *binding = keys.clone();
        }
    }
    bindings
}

fn validate(bindings: &BTreeMap<String, Vec<String>>) -> Result<(), String> {
    let mut used = HashMap::new();
    for (id, label, _) in ACTIONS {
        for key in &bindings[*id] {
            let Some((symbol, modifiers)) = gtk::accelerator_parse(key) else {
                return Err(format!("Invalid shortcut for {label}: {key}"));
            };
            if symbol == gdk::Key::VoidSymbol {
                return Err(format!("Invalid shortcut for {label}: {key}"));
            }
            if let Some(first) = used.insert((symbol, modifiers), label) {
                let binding = gtk::accelerator_get_label(symbol, modifiers);
                return Err(format!(
                    "{binding} is assigned to both {first} and {label}."
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn apply(app: &gtk::Application, config: &Config) {
    let bindings = effective(config);
    let bindings = if let Err(err) = validate(&bindings) {
        eprintln!("Veyl: {err} Using default shortcuts.");
        defaults()
    } else {
        bindings
    };
    for (id, keys) in bindings {
        let keys = keys.iter().map(String::as_str).collect::<Vec<_>>();
        app.set_accels_for_action(&format!("win.{id}"), &keys);
    }
}

pub(super) struct Editor {
    entries: Vec<(&'static str, gtk::Entry)>,
    initial: BTreeMap<String, Vec<String>>,
    existing: BTreeMap<String, Vec<String>>,
}

impl Editor {
    pub(super) fn new(body: &gtk::Box, config: &Config) -> Self {
        let bindings = effective(config);
        let mut entries = Vec::new();
        for (id, label, _) in ACTIONS {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            let text = gtk::Label::new(Some(label));
            text.set_xalign(0.0);
            text.set_hexpand(true);
            let entry = gtk::Entry::new();
            entry.update_property(&[Property::Label(label)]);
            entry.set_text(&bindings[*id].join(", "));
            let recorder = gtk::EventControllerKey::new();
            recorder.set_propagation_phase(gtk::PropagationPhase::Capture);
            recorder.connect_key_pressed(glib::clone!(
                #[weak]
                entry,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_, key, _, modifiers| {
                    let mask = modifiers & gtk::accelerator_get_default_mod_mask();
                    let command_modifiers = gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::ALT_MASK
                        | gdk::ModifierType::SUPER_MASK
                        | gdk::ModifierType::META_MASK;
                    if !mask.intersects(command_modifiers)
                        && !(gdk::Key::F1..=gdk::Key::F35).contains(&key)
                    {
                        return glib::Propagation::Proceed;
                    }
                    if matches!(
                        key,
                        gdk::Key::Control_L
                            | gdk::Key::Control_R
                            | gdk::Key::Shift_L
                            | gdk::Key::Shift_R
                            | gdk::Key::Alt_L
                            | gdk::Key::Alt_R
                            | gdk::Key::Super_L
                            | gdk::Key::Super_R
                            | gdk::Key::Meta_L
                            | gdk::Key::Meta_R
                    ) {
                        return glib::Propagation::Stop;
                    }
                    let key = if key == gdk::Key::ISO_Left_Tab {
                        gdk::Key::Tab
                    } else {
                        key.to_lower()
                    };
                    entry.set_text(&gtk::accelerator_name(key, mask));
                    glib::Propagation::Stop
                }
            ));
            entry.add_controller(recorder);
            row.add_css_class("veyl-settings-row");
            row.append(&text);
            row.append(&entry);
            body.append(&row);
            entries.push((*id, entry));
        }

        Self {
            entries,
            initial: bindings,
            existing: config.shortcuts.clone(),
        }
    }

    pub(super) fn reset(&self) {
        let defaults = defaults();
        for (id, entry) in &self.entries {
            entry.set_text(&defaults[*id].join(", "));
        }
    }

    pub(super) fn draft(&self) -> Result<BTreeMap<String, Vec<String>>, String> {
        let bindings = self
            .entries
            .iter()
            .map(|(id, entry)| {
                (
                    (*id).to_owned(),
                    entry
                        .text()
                        .split(',')
                        .map(str::trim)
                        .filter(|key| !key.is_empty())
                        .map(str::to_owned)
                        .collect(),
                )
            })
            .collect();
        if bindings == self.initial {
            return Ok(self.existing.clone());
        }
        validate(&bindings)?;
        let defaults = defaults();
        let mut overrides = self.existing.clone();
        for (id, keys) in bindings {
            if keys == defaults[&id] {
                overrides.remove(&id);
            } else {
                overrides.insert(id, keys);
            }
        }
        Ok(overrides)
    }
}
