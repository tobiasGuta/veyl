use crate::config::{Config, Profile};
use gtk::glib;
use gtk::prelude::*;
use std::{cell::RefCell, collections::BTreeMap, path::Path, rc::Rc};

pub(super) struct Edit {
    pub profiles: BTreeMap<String, Profile>,
    pub default: String,
    pub renamed: BTreeMap<String, String>,
}

struct Row {
    original: Option<(String, Profile)>,
    name: gtk::Entry,
    fields: [gtk::Entry; 3],
    scheme: gtk::DropDown,
}

impl Row {
    fn value(&self, schemes: &[Option<String>]) -> Profile {
        let field = |index: usize| {
            let text = self.fields[index].text();
            (!text.is_empty()).then(|| text.to_string())
        };
        Profile {
            shell: field(0),
            directory: field(1).map(Into::into),
            tab_name: field(2),
            color_scheme: schemes
                .get(self.scheme.selected() as usize)
                .cloned()
                .flatten(),
        }
    }
}

pub(super) struct Editor {
    pub default: gtk::DropDown,
    names: gtk::StringList,
    schemes: Vec<Option<String>>,
    rows: RefCell<Vec<Row>>,
    body: gtk::Box,
    unavailable_default: Option<String>,
}

impl Editor {
    pub(super) fn new(page: &gtk::Box, config: &Config) -> Rc<Self> {
        let unavailable_default = (!config.default_profile.is_empty()
            && !config.profiles.contains_key(&config.default_profile))
        .then(|| config.default_profile.clone());
        let names = gtk::StringList::new(&["System shell"]);
        if let Some(name) = &unavailable_default {
            names.append(&format!("{name} (unavailable)"));
        }
        let default = gtk::DropDown::new(Some(names.clone()), gtk::Expression::NONE);
        super::settings::append_row(page, "Default profile", default.upcast_ref());
        let add = gtk::Button::with_label("Add profile");
        add.set_halign(gtk::Align::Start);
        page.append(&add);
        let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
        page.append(&body);
        let mut schemes = vec![None];
        schemes.extend(super::themes::names(config).into_iter().map(Some));
        for name in config
            .profiles
            .values()
            .filter_map(|profile| profile.color_scheme.as_ref())
        {
            if !schemes.iter().any(|scheme| scheme.as_ref() == Some(name)) {
                schemes.push(Some(name.clone()));
            }
        }
        let editor = Rc::new(Self {
            default,
            names,
            schemes,
            rows: RefCell::new(Vec::new()),
            body,
            unavailable_default,
        });
        for (name, profile) in &config.profiles {
            editor.append(name, profile, Some((name.clone(), profile.clone())));
        }
        if !config.default_profile.is_empty() {
            let index = editor
                .rows
                .borrow()
                .iter()
                .position(|row| row.name.text() == config.default_profile)
                .map_or(editor.rows.borrow().len() + 1, |index| index + 1);
            editor.default.set_selected(index as u32);
        }
        add.connect_clicked(glib::clone!(
            #[weak]
            editor,
            move |_| {
                let name = editor.available_name("New profile");
                editor.append(&name, &Profile::default(), None);
            }
        ));
        editor
    }

    fn available_name(&self, base: &str) -> String {
        let rows = self.rows.borrow();
        std::iter::once(base.to_owned())
            .chain((2..).map(|n| format!("{base} {n}")))
            .find(|name| rows.iter().all(|row| row.name.text() != name.as_str()))
            .unwrap()
    }

    fn append(self: &Rc<Self>, name: &str, profile: &Profile, original: Option<(String, Profile)>) {
        let expander = gtk::Expander::new(Some(name));
        expander.set_expanded(original.is_none());
        expander.add_css_class("veyl-settings-row");
        let form = gtk::Box::new(gtk::Orientation::Vertical, 6);
        let name_entry = gtk::Entry::builder().text(name).build();
        super::settings::append_row(&form, "Profile name", name_entry.upcast_ref());
        let fields = [
            (
                "Shell executable",
                profile.shell.as_deref().unwrap_or(""),
                "System shell",
            ),
            (
                "Starting folder",
                profile
                    .directory
                    .as_ref()
                    .and_then(|path| path.to_str())
                    .unwrap_or(""),
                "Home folder",
            ),
            (
                "Tab label",
                profile.tab_name.as_deref().unwrap_or(""),
                "Profile name",
            ),
        ]
        .map(|(label, text, placeholder)| {
            let entry = gtk::Entry::builder()
                .text(text)
                .placeholder_text(placeholder)
                .build();
            super::settings::append_row(&form, label, entry.upcast_ref());
            entry
        });
        let scheme_labels = self
            .schemes
            .iter()
            .map(|scheme| match scheme.as_deref() {
                None => "Inherit default scheme",
                Some("") => "VTE default",
                Some(name) => name,
            })
            .collect::<Vec<_>>();
        let scheme = gtk::DropDown::from_strings(&scheme_labels);
        let selected = self
            .schemes
            .iter()
            .position(|scheme| *scheme == profile.color_scheme)
            .unwrap_or_default();
        scheme.set_selected(selected as u32);
        super::settings::append_row(&form, "Color scheme", scheme.upcast_ref());
        let duplicate = gtk::Button::with_label("Duplicate profile");
        duplicate.set_halign(gtk::Align::Start);
        form.append(&duplicate);
        expander.set_child(Some(&form));
        self.body.append(&expander);

        let index = self.rows.borrow().len();
        let selected = self.default.selected();
        self.names.splice((index + 1) as u32, 0, &[name]);
        if selected == index as u32 + 1 {
            self.default.set_selected(selected + 1);
        }
        self.rows.borrow_mut().push(Row {
            original,
            name: name_entry.clone(),
            fields,
            scheme,
        });

        name_entry.connect_changed(glib::clone!(
            #[weak(rename_to = editor)]
            self,
            #[weak]
            expander,
            move |entry| {
                let text = entry.text();
                let label = if text.trim().is_empty() {
                    "Unnamed profile"
                } else {
                    text.trim()
                };
                expander.set_label(Some(label));
                let selected = editor.default.selected();
                editor.names.splice((index + 1) as u32, 1, &[label]);
                editor.default.set_selected(selected);
            }
        ));
        duplicate.connect_clicked(glib::clone!(
            #[weak(rename_to = editor)]
            self,
            move |_| {
                let (base, profile) = {
                    let rows = editor.rows.borrow();
                    (
                        format!("{} copy", rows[index].name.text().trim()),
                        rows[index].value(&editor.schemes),
                    )
                };
                editor.append(&editor.available_name(&base), &profile, None);
            }
        ));
    }

    pub(super) fn draft(&self) -> Result<Edit, String> {
        let mut edit = Edit {
            profiles: BTreeMap::new(),
            default: String::new(),
            renamed: BTreeMap::new(),
        };
        let rows = self.rows.borrow();
        let selected = self.default.selected() as usize;
        for (index, row) in rows.iter().enumerate() {
            let raw_name = row.name.text();
            let mut name = raw_name.trim().to_owned();
            let mut value = row.value(&self.schemes);
            if let Some((original, profile)) = &row.original {
                if original == raw_name.as_str() {
                    name = original.clone();
                }
                // Preserve explicit empty values and fields this editor cannot represent.
                if row.fields[0].text() == profile.shell.as_deref().unwrap_or("") {
                    value.shell = profile.shell.clone();
                }
                if row.fields[1].text()
                    == profile
                        .directory
                        .as_ref()
                        .and_then(|p| p.to_str())
                        .unwrap_or("")
                {
                    value.directory = profile.directory.clone();
                }
                if row.fields[2].text() == profile.tab_name.as_deref().unwrap_or("") {
                    value.tab_name = profile.tab_name.clone();
                }
                if original != &name {
                    edit.renamed.insert(original.clone(), name.clone());
                }
            }
            if name.is_empty() && row.original.as_ref().is_none_or(|(old, _)| !old.is_empty()) {
                return Err("Enter a name for every profile.".into());
            }
            let original = row.original.as_ref().map(|(_, profile)| profile);
            if value.shell.as_deref() != original.and_then(|p| p.shell.as_deref())
                && value
                    .shell
                    .as_deref()
                    .is_some_and(|path| !Path::new(path).is_absolute())
            {
                return Err(format!("Use an absolute shell executable path for {name}."));
            }
            if value.directory.as_deref() != original.and_then(|p| p.directory.as_deref())
                && value
                    .directory
                    .as_deref()
                    .is_some_and(|path| !path.is_absolute() && !path.starts_with("~"))
            {
                return Err(format!(
                    "Use an absolute path or a path starting with ~/ for {name}."
                ));
            }
            if selected == index + 1 {
                edit.default.clone_from(&name);
            }
            if edit.profiles.insert(name.clone(), value).is_some() {
                return Err(format!("A profile named {name} already exists."));
            }
        }
        if selected == rows.len() + 1 {
            edit.default = self.unavailable_default.clone().unwrap_or_default();
        }
        Ok(edit)
    }
}
