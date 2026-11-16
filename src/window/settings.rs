use super::{appearance_color, apply_terminal_appearance, shortcuts, themes};
use crate::config::{Config, CursorShape};
use gtk::accessible::Property;
use gtk::prelude::*;
use gtk::{glib, pango};
use std::{io, rc::Rc};
use vte4::prelude::*;

const CURSORS: [CursorShape; 3] = [
    CursorShape::Block,
    CursorShape::Beam,
    CursorShape::Underline,
];

struct Form {
    font: gtk::FontDialogButton,
    size: gtk::SpinButton,
    foreground: gtk::ColorDialogButton,
    background: gtk::ColorDialogButton,
    cursor: gtk::DropDown,
    scheme: gtk::ListBox,
    blur: gtk::Switch,
    opacity: gtk::Scale,
    profile: gtk::DropDown,
    profile_names: Vec<String>,
    scheme_names: Vec<String>,
    progress: gtk::Switch,
    padding_x: gtk::SpinButton,
    padding_y: gtk::SpinButton,
    preview: vte4::Terminal,
    original: Config,
}

impl Form {
    fn draft(&self) -> Config {
        Config {
            font_family: self
                .font
                .font_desc()
                .and_then(|font| font.family())
                .map_or_else(
                    || self.original.font_family.clone(),
                    |family| family.to_string(),
                ),
            font_size: self.size.value_as_int() as u16,
            foreground: color_setting(&self.foreground, &self.original.foreground, "#e8eaf0"),
            background: color_setting(&self.background, &self.original.background, "#10121a"),
            cursor_shape: CURSORS
                .get(self.cursor.selected() as usize)
                .copied()
                .unwrap_or_default(),
            tab_progress: self.progress.is_active(),
            blur: self.blur.is_active(),
            opacity: if self.opacity.value() == self.original.opacity * 100.0 {
                self.original.opacity
            } else {
                self.opacity.value() / 100.0
            },
            default_profile: self
                .profile_names
                .get(self.profile.selected() as usize)
                .cloned()
                .unwrap_or_default(),
            color_scheme: self
                .scheme_names
                .get(
                    self.scheme
                        .selected_row()
                        .map_or(0, |row| row.index() as usize),
                )
                .cloned()
                .unwrap_or_default(),
            padding_x: self.padding_x.value_as_int() as u16,
            padding_y: self.padding_y.value_as_int() as u16,
            ..self.original.clone()
        }
    }

    fn load(&self, config: &Config) {
        self.load_appearance(config);
        self.select_scheme(&config.color_scheme);
        self.select_profile(&config.default_profile);
    }

    fn load_appearance(&self, config: &Config) {
        let mut font = pango::FontDescription::new();
        font.set_family(&config.font_family);
        self.font.set_font_desc(&font);
        self.size.set_value(f64::from(config.font_size));
        self.foreground
            .set_rgba(&appearance_color(&config.foreground, "#e8eaf0"));
        self.background
            .set_rgba(&appearance_color(&config.background, "#10121a"));
        self.cursor.set_selected(
            CURSORS
                .iter()
                .position(|shape| *shape == config.cursor_shape)
                .unwrap_or_default() as u32,
        );
        self.blur.set_active(config.blur);
        self.opacity.set_value(config.opacity * 100.0);
        self.progress.set_active(config.tab_progress);
        self.padding_x.set_value(f64::from(config.padding_x));
        self.padding_y.set_value(f64::from(config.padding_y));
    }

    fn select_scheme(&self, name: &str) {
        let index = self
            .scheme_names
            .iter()
            .position(|item| item == name)
            .unwrap_or_default();
        self.scheme
            .select_row(self.scheme.row_at_index(index as i32).as_ref());
    }

    fn select_profile(&self, name: &str) {
        let index = self
            .profile_names
            .iter()
            .position(|item| item == name)
            .unwrap_or_default();
        self.profile.set_selected(index as u32);
    }

    fn refresh_preview(&self) {
        let draft = self.draft();
        apply_terminal_appearance(&self.preview, &draft);
        self.preview
            .set_color_background(&appearance_color(&draft.background, "#10121a"));
        self.preview.set_margin_start(i32::from(draft.padding_x));
        self.preview.set_margin_end(i32::from(draft.padding_x));
        self.preview.set_margin_top(i32::from(draft.padding_y));
        self.preview.set_margin_bottom(i32::from(draft.padding_y));
    }
}

fn color_setting(button: &gtk::ColorDialogButton, original: &str, fallback: &str) -> String {
    let color = button.rgba();
    if color == appearance_color(original, fallback) {
        original.to_owned()
    } else {
        color.to_str().to_string()
    }
}

pub(super) struct Workspace {
    pub surface: gtk::Box,
    pub pages: gtk::Stack,
}

pub(super) fn build(
    original: &Config,
    blur_supported: bool,
    save: impl Fn(Config) -> io::Result<()> + 'static,
    dismiss: impl Fn() + Clone + 'static,
) -> Workspace {
    let color_dialog = gtk::ColorDialog::builder().with_alpha(false).build();
    let font = gtk::FontDialogButton::new(Some(gtk::FontDialog::new()));
    font.set_level(gtk::FontLevel::Family);
    let mut scheme_names = themes::names(original);
    if !scheme_names.contains(&original.color_scheme) {
        scheme_names.push(original.color_scheme.clone());
    }
    let scheme = gtk::ListBox::new();
    scheme.add_css_class("veyl-schemes");
    for name in &scheme_names {
        let item = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        if let Some(colors) = themes::resolve(original, name)
            .as_ref()
            .and_then(themes::palette)
        {
            let swatches = gtk::DrawingArea::new();
            swatches.set_content_width(80);
            swatches.set_content_height(20);
            swatches.set_valign(gtk::Align::Center);
            swatches.set_draw_func(move |_, cr, _, _| {
                for (index, color) in colors.iter().enumerate() {
                    cr.set_source_rgb(
                        color.red().into(),
                        color.green().into(),
                        color.blue().into(),
                    );
                    cr.rectangle((index % 8 * 10) as f64, (index / 8 * 10) as f64, 9.0, 9.0);
                    let _ = cr.fill();
                }
            });
            item.append(&swatches);
        }
        let label = gtk::Label::new(Some(if name.is_empty() { "VTE default" } else { name }));
        label.set_wrap(true);
        label.set_xalign(0.0);
        item.append(&label);
        scheme.append(&item);
    }
    let mut profile_names = vec![String::new()];
    profile_names.extend(
        original
            .profiles
            .keys()
            .filter(|name| !name.is_empty())
            .cloned(),
    );
    if !profile_names.contains(&original.default_profile) {
        profile_names.push(original.default_profile.clone());
    }
    let profile_labels = profile_names
        .iter()
        .map(|name| {
            if name.is_empty() {
                "System shell"
            } else {
                name.as_str()
            }
        })
        .collect::<Vec<_>>();
    let profile = gtk::DropDown::from_strings(&profile_labels);
    let opacity = gtk::Scale::with_range(gtk::Orientation::Horizontal, 40.0, 100.0, 1.0);
    opacity.set_width_request(135);
    opacity.set_draw_value(true);
    opacity.set_value_pos(gtk::PositionType::Right);
    opacity.set_format_value_func(|_, value| format!("{value:.0}%"));
    let form = Rc::new(Form {
        scheme,
        blur: gtk::Switch::new(),
        opacity,
        profile,
        profile_names,
        scheme_names,
        progress: gtk::Switch::new(),
        font,
        size: gtk::SpinButton::with_range(7.0, 32.0, 1.0),
        foreground: gtk::ColorDialogButton::new(Some(color_dialog.clone())),
        background: gtk::ColorDialogButton::new(Some(color_dialog)),
        cursor: gtk::DropDown::from_strings(&["Block", "Beam", "Underline"]),
        padding_x: gtk::SpinButton::with_range(0.0, 48.0, 1.0),
        padding_y: gtk::SpinButton::with_range(0.0, 48.0, 1.0),
        preview: vte4::Terminal::new(),
        original: original.clone(),
    });
    form.load(original);
    form.preview.set_size(30, 4);
    form.preview.set_can_focus(false);
    form.preview
        .update_property(&[Property::Label("Terminal appearance preview")]);
    form.preview
        .feed(b"$ printf 'Hello, Veyl!\\n'\r\n\x1b[32mHello, Veyl!\x1b[0m\r\n$ ");
    form.refresh_preview();

    let surface = gtk::Box::new(gtk::Orientation::Vertical, 0);
    surface.add_css_class("veyl-settings");
    let layout = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    layout.set_vexpand(true);
    surface.append(&layout);

    let pages = gtk::Stack::new();
    pages.set_hexpand(true);
    let sidebar = gtk::StackSidebar::new();
    sidebar.set_stack(&pages);
    sidebar.add_css_class("veyl-settings-nav");
    sidebar.update_property(&[Property::Label("Settings sections")]);
    layout.append(&sidebar);
    layout.append(&pages);

    let appearance = page(&pages, "appearance", "Appearance", None);
    let rows: [(&str, &gtk::Widget, &str); 10] = [
        ("Font family", form.font.upcast_ref(), "font-desc"),
        ("Font size (pt)", form.size.upcast_ref(), "value"),
        ("Text color", form.foreground.upcast_ref(), "rgba"),
        ("Background tint", form.background.upcast_ref(), "rgba"),
        ("Frosted background", form.blur.upcast_ref(), "active"),
        ("Background opacity", form.opacity.upcast_ref(), "value"),
        ("Cursor shape", form.cursor.upcast_ref(), "selected"),
        ("Show tab progress", form.progress.upcast_ref(), "active"),
        (
            "Horizontal padding (px)",
            form.padding_x.upcast_ref(),
            "value",
        ),
        (
            "Vertical padding (px)",
            form.padding_y.upcast_ref(),
            "value",
        ),
    ];
    let preview_frame = gtk::Frame::new(Some("Terminal preview"));
    preview_frame.set_child(Some(
        &gtk::ScrolledWindow::builder()
            .min_content_height(108)
            .child(&form.preview)
            .build(),
    ));
    appearance.append(&preview_frame);
    for (text, control, property) in rows {
        append_row(&appearance, text, control);
        control.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                form,
                move |_, _| form.refresh_preview()
            ),
        );
    }
    if !blur_supported {
        let help =
            description("Blur is unavailable on this display. Turn it off to adjust transparency.");
        help.set_visible(form.blur.is_active());
        form.opacity.set_sensitive(!form.blur.is_active());
        form.blur.connect_active_notify(glib::clone!(
            #[weak]
            help,
            #[weak(rename_to = opacity)]
            form.opacity,
            move |blur| {
                help.set_visible(blur.is_active());
                opacity.set_sensitive(!blur.is_active());
            }
        ));
        appearance.append(&help);
    }

    let schemes = page(
        &pages,
        "schemes",
        "Color schemes",
        Some("Choose the ANSI palette for terminals that use the default scheme."),
    );
    schemes.append(&form.scheme);
    form.scheme.connect_row_selected(glib::clone!(
        #[weak]
        form,
        move |_, _| form.refresh_preview()
    ));
    let sample = vte4::Terminal::new();
    sample.set_can_focus(false);
    sample.set_size(24, 2);
    sample.update_property(&[Property::Label("Color scheme preview")]);
    let sample_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(74)
        .child(&sample)
        .build();
    schemes.append(&sample_scroll);
    sample.feed(b"\x1b[31mRed \x1b[32mGreen \x1b[33mYellow \x1b[34mBlue\x1b[0m");
    let refresh_sample = |form: &Form, sample: &vte4::Terminal| {
        let draft = form.draft();
        apply_terminal_appearance(sample, &draft);
        sample.set_color_background(&appearance_color(&draft.background, "#10121a"));
    };
    refresh_sample(&form, &sample);
    form.scheme.connect_row_selected(glib::clone!(
        #[weak]
        form,
        #[weak]
        sample,
        move |_, _| refresh_sample(&form, &sample)
    ));
    pages.connect_visible_child_name_notify(glib::clone!(
        #[weak]
        form,
        #[weak]
        sample,
        move |_| refresh_sample(&form, &sample)
    ));

    let profiles = page(
        &pages,
        "profiles",
        "Profiles",
        Some("The + button opens the default profile. Open other profiles from the tab menu."),
    );
    append_row(&profiles, "Default profile", form.profile.upcast_ref());
    let shortcuts = page(
        &pages,
        "shortcuts",
        "Keyboard shortcuts",
        Some("Press a key combination in a field. Clear a field to unbind that action."),
    );
    let editor = Rc::new(shortcuts::Editor::new(&shortcuts, original));

    let error = description("");
    error.add_css_class("error");
    error.set_visible(false);
    surface.append(&error);

    let footer = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    footer.add_css_class("veyl-settings-footer");
    let reset = gtk::Button::with_label("Reset to defaults");
    reset.connect_clicked(glib::clone!(
        #[weak]
        form,
        #[weak]
        editor,
        #[weak]
        pages,
        move |_| {
            match pages.visible_child_name().as_deref() {
                Some("shortcuts") => editor.reset(),
                Some("schemes") => form.select_scheme(""),
                Some("profiles") => form.select_profile(""),
                _ => form.load_appearance(&Config::default()),
            }
        }
    ));
    footer.append(&reset);
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    footer.append(&spacer);
    let apply = gtk::Button::with_label("Save");
    apply.add_css_class("suggested-action");
    apply.connect_clicked({
        let dismiss = dismiss.clone();
        let pages = pages.downgrade();
        move |_| {
            for field in [&form.size, &form.padding_x, &form.padding_y] {
                field.update();
            }
            let shortcuts = match editor.draft() {
                Ok(shortcuts) => shortcuts,
                Err(message) => {
                    if let Some(pages) = pages.upgrade() {
                        pages.set_visible_child_name("shortcuts");
                    }
                    error.set_text(&message);
                    error.set_visible(true);
                    return;
                }
            };
            let mut draft = form.draft();
            draft.shortcuts = shortcuts;
            match save(draft) {
                Ok(()) => dismiss(),
                Err(message) => {
                    error.set_text(&format!("Could not save settings. {message}"));
                    error.set_visible(true);
                }
            }
        }
    });
    footer.append(&apply);
    let cancel = gtk::Button::with_label("Discard changes");
    cancel.connect_clicked(move |_| dismiss());
    footer.append(&cancel);
    surface.append(&footer);

    Workspace { surface, pages }
}

fn page(pages: &gtk::Stack, name: &str, title: &str, help: Option<&str>) -> gtk::Box {
    let body = gtk::Box::new(gtk::Orientation::Vertical, 6);
    body.add_css_class("veyl-settings-page");
    let heading = description(title);
    heading.add_css_class("veyl-settings-heading");
    body.append(&heading);
    if let Some(help) = help {
        let label = description(help);
        label.add_css_class("veyl-hint");
        body.append(&label);
    }
    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&body)
        .build();
    pages.add_titled(&scroll, Some(name), title);
    body
}

fn description(text: &str) -> gtk::Label {
    let label = gtk::Label::new(Some(text));
    label.set_xalign(0.0);
    label.set_wrap(true);
    label
}

fn append_row(body: &gtk::Box, text: &str, control: &gtk::Widget) {
    let row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    row.add_css_class("veyl-settings-row");
    let label = description(text);
    label.set_hexpand(true);
    control.set_valign(gtk::Align::Center);
    control.update_property(&[Property::Label(text)]);
    row.append(&label);
    row.append(control);
    body.append(&row);
}
