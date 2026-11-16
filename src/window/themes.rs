use crate::config::{ColorScheme, Config};
use gtk::gdk::RGBA;

pub(super) fn names(config: &Config) -> Vec<String> {
    let mut names = vec![String::new(), "Midnight".into(), "Warm dusk".into()];
    names.extend(
        config
            .schemes
            .keys()
            .filter(|name| !name.is_empty() && !matches!(name.as_str(), "Midnight" | "Warm dusk"))
            .cloned(),
    );
    names
}

pub(super) fn resolve(config: &Config, name: &str) -> Option<ColorScheme> {
    if let Some(scheme) = config.schemes.get(name).filter(|_| !name.is_empty()) {
        return Some(scheme.clone());
    }
    let (ansi, cursor, selection) = match name {
        "Midnight" => (
            [
                "#161924", "#f17883", "#91d7a2", "#dfc27a", "#81aaff", "#c8a1ed", "#7dcedb",
                "#dae2f2", "#777f99", "#ff9da6", "#aae9b2", "#f5d995", "#a9c5ff", "#dfbaff",
                "#a3e2ef", "#ffffff",
            ],
            "#b8cdff",
            "#3d5180",
        ),
        "Warm dusk" => (
            [
                "#211b20", "#ff8278", "#9cce88", "#e2bd75", "#96b5e6", "#db9bc6", "#8acfc0",
                "#efe0d2", "#88767d", "#ffa79c", "#badfa4", "#f3d298", "#b7d2fb", "#efb7dc",
                "#aee1d5", "#fff5e7",
            ],
            "#edd09e",
            "#734d5c",
        ),
        _ => return None,
    };
    Some(ColorScheme {
        ansi: ansi.map(str::to_owned),
        cursor: Some(cursor.into()),
        selection: Some(selection.into()),
    })
}

pub(super) fn palette(scheme: &ColorScheme) -> Option<Vec<RGBA>> {
    scheme
        .ansi
        .iter()
        .map(|color| {
            let mut color = RGBA::parse(color).ok()?;
            color.set_alpha(1.0);
            Some(color)
        })
        .collect()
}
