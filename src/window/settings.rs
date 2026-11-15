use super::{appearance_color, apply_terminal_appearance, themes};
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
    scheme: gtk::DropDown,
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
            color_scheme: self
                .scheme_names
                .get(self.scheme.selected() as usize)
                .cloned()
                .unwrap_or_default(),
            padding_x: self.padding_x.value_as_int() as u16,
            padding_y: self.padding_y.value_as_int() as u16,
            ..self.original.clone()
        }
    }

    fn load(&self, config: &Config) {
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
        self.scheme.set_selected(
            self.scheme_names
                .iter()
                .position(|name| *name == config.color_scheme)
                .unwrap_or_default() as u32,
        );
        self.progress.set_active(config.tab_progress);
        self.padding_x.set_value(f64::from(config.padding_x));
        self.padding_y.set_value(f64::from(config.padding_y));
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

pub(super) fn show(
    parent: &gtk::ApplicationWindow,
    original: &Config,
    save: impl Fn(Config) -> io::Result<()> + 'static,
) {
    let window = gtk::Window::builder()
        .title("Veyl Settings")
        .transient_for(parent)
        .destroy_with_parent(true)
        .modal(true)
        .default_width(520)
        .default_height(565)
        .build();
    let header = gtk::HeaderBar::new();
    window.set_titlebar(Some(&header));

    let color_dialog = gtk::ColorDialog::builder().with_alpha(false).build();
    let font = gtk::FontDialogButton::new(Some(gtk::FontDialog::new()));
    font.set_level(gtk::FontLevel::Family);
    let mut scheme_names = themes::names(original);
    if !scheme_names.contains(&original.color_scheme) {
        scheme_names.push(original.color_scheme.clone());
    }
    let labels = scheme_names
        .iter()
        .map(|name| {
            if name.is_empty() {
                "Default"
            } else {
                name.as_str()
            }
        })
        .collect::<Vec<_>>();
    let scheme = gtk::DropDown::from_strings(&labels);
    let form = Rc::new(Form {
        scheme,
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

    let body = gtk::Box::new(gtk::Orientation::Vertical, 14);
    body.set_margin_top(16);
    body.set_margin_bottom(16);
    body.set_margin_start(18);
    body.set_margin_end(18);
    let rows: [(&str, &gtk::Widget, &str); 9] = [
        ("Font family", form.font.upcast_ref(), "font-desc"),
        ("Font size (pt)", form.size.upcast_ref(), "value"),
        ("Text color", form.foreground.upcast_ref(), "rgba"),
        ("Background tint", form.background.upcast_ref(), "rgba"),
        ("Cursor shape", form.cursor.upcast_ref(), "selected"),
        ("Color scheme", form.scheme.upcast_ref(), "selected"),
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
    for (text, control, property) in rows {
        let row = gtk::Box::new(gtk::Orientation::Horizontal, 16);
        let label = gtk::Label::new(Some(text));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        control.update_property(&[Property::Label(text)]);
        row.append(&label);
        row.append(control);
        body.append(&row);
        control.connect_notify_local(
            Some(property),
            glib::clone!(
                #[weak]
                form,
                move |_, _| {
                    form.refresh_preview();
                }
            ),
        );
    }

    let preview_frame = gtk::Frame::new(Some("Preview"));
    let preview_scroll = gtk::ScrolledWindow::builder()
        .min_content_height(112)
        .hscrollbar_policy(gtk::PolicyType::Automatic)
        .vscrollbar_policy(gtk::PolicyType::Automatic)
        .child(&form.preview)
        .build();
    preview_frame.set_child(Some(&preview_scroll));
    body.append(&preview_frame);

    let error = gtk::Label::new(None);
    error.set_wrap(true);
    error.set_visible(false);
    error.set_xalign(0.0);
    error.add_css_class("error");
    body.append(&error);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&body)
        .build();
    window.set_child(Some(&scroll));

    let reset = gtk::Button::with_label("Reset to defaults");
    reset.connect_clicked(glib::clone!(
        #[weak]
        form,
        move |_| form.load(&Config::default())
    ));
    body.append(&reset);

    let cancel = gtk::Button::with_label("Cancel");
    cancel.connect_clicked(glib::clone!(
        #[weak]
        window,
        move |_| window.close()
    ));
    header.pack_start(&cancel);

    let save_button = gtk::Button::with_label("Save");
    save_button.add_css_class("suggested-action");
    save_button.connect_clicked(glib::clone!(
        #[weak]
        window,
        move |_| {
            for field in [&form.size, &form.padding_x, &form.padding_y] {
                field.update();
            }
            match save(form.draft()) {
                Ok(()) => window.close(),
                Err(err) => {
                    error.set_text(&format!("Could not save settings. {err}"));
                    error.set_visible(true);
                }
            }
        }
    ));
    header.pack_end(&save_button);
    window.present();
}
