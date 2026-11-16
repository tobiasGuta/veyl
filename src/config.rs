use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CursorShape {
    #[default]
    Block,
    Beam,
    Underline,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub shell: Option<String>,
    pub directory: Option<PathBuf>,
    pub tab_name: Option<String>,
    pub color_scheme: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ColorScheme {
    pub ansi: [String; 16],
    pub cursor: Option<String>,
    pub selection: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub opacity: f64,
    pub blur: bool,
    pub font_family: String,
    pub font_size: u16,
    pub scrollback_lines: i64,
    pub background: String,
    pub foreground: String,
    pub cursor_shape: CursorShape,
    pub padding_x: u16,
    pub padding_y: u16,
    pub color_scheme: String,
    pub schemes: BTreeMap<String, ColorScheme>,
    pub default_profile: String,
    pub profiles: BTreeMap<String, Profile>,
    pub shortcuts: BTreeMap<String, Vec<String>>,
    pub tab_progress: bool,
    pub confirm_multiline_paste: bool,
    pub bash_prompt: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            opacity: 0.72,
            blur: true,
            font_family: "Cascadia Mono".to_owned(),
            font_size: 12,
            scrollback_lines: 10_000,
            background: "#10121a".to_owned(),
            foreground: "#e8eaf0".to_owned(),
            cursor_shape: CursorShape::Block,
            padding_x: 10,
            padding_y: 8,
            color_scheme: String::new(),
            schemes: BTreeMap::new(),
            default_profile: String::new(),
            profiles: BTreeMap::from([(
                "Bash".into(),
                Profile {
                    shell: Some("/bin/bash".into()),
                    ..Profile::default()
                },
            )]),
            shortcuts: BTreeMap::new(),
            tab_progress: true,
            confirm_multiline_paste: true,
            bash_prompt: None,
        }
    }
}

#[derive(Debug)]
pub struct ConfigLoadError {
    pub path: Option<PathBuf>,
    pub cause: io::Error,
}

impl Config {
    pub fn background_style(&self, blur_supported: bool) -> (f64, bool) {
        match (self.blur, blur_supported) {
            (true, false) => (1.0, false),
            (true, true) => (self.opacity, self.opacity < 1.0),
            (false, _) => (self.opacity, false),
        }
    }

    pub fn normalize(&mut self) {
        if !self.opacity.is_finite() {
            self.opacity = Self::default().opacity;
        }
        self.opacity = self.opacity.clamp(0.40, 1.0);
        self.font_size = self.font_size.clamp(7, 32);
        self.padding_x = self.padding_x.min(48);
        self.padding_y = self.padding_y.min(48);
        self.scrollback_lines = self.scrollback_lines.clamp(100, 200_000);
        if self.font_family.trim().is_empty() {
            self.font_family = Self::default().font_family;
        }
        // These colors are parsed by GTK. Invalid values fall back at render time.
    }

    pub fn path() -> Option<PathBuf> {
        let root = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|v| PathBuf::from(v).join(".config")))?;
        Some(root.join("veyl").join("config.toml"))
    }

    pub fn load() -> Result<Self, ConfigLoadError> {
        let path = Self::path().ok_or_else(|| ConfigLoadError {
            path: None,
            cause: io::Error::other("No HOME or XDG_CONFIG_HOME is set."),
        })?;
        let result = match fs::read_to_string(&path) {
            Ok(contents) => toml::from_str::<Self>(&contents).map_err(io::Error::other),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(Self::default()),
            Err(err) => Err(err),
        };
        let mut config = result.map_err(|cause| ConfigLoadError {
            path: Some(path),
            cause,
        })?;
        config.normalize();
        Ok(config)
    }

    pub fn save_changes(
        &self,
        previous: &Self,
        renamed: &BTreeMap<String, String>,
    ) -> io::Result<()> {
        let path = Self::path().ok_or_else(|| io::Error::other("no user config directory"))?;
        // Keep a dotfiles symlink pointing at its original destination.
        let path = match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.is_symlink() => fs::canonicalize(path)?,
            Ok(_) => path,
            Err(err) if err.kind() == io::ErrorKind::NotFound => path,
            Err(err) => return Err(err),
        };
        let source = match fs::read_to_string(&path) {
            Ok(contents) => contents,
            Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
            Err(err) => return Err(err),
        };
        let content = self.update_document(previous, &source, renamed)?;
        if content != source {
            write_settings(&path, &content, 0o600)?;
        }
        Ok(())
    }

    fn update_document(
        &self,
        previous: &Self,
        source: &str,
        renamed: &BTreeMap<String, String>,
    ) -> io::Result<String> {
        let mut document = source
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        let mut previous = previous.clone();
        if let Some(profiles) = document
            .get_mut("profiles")
            .and_then(toml_edit::Item::as_table_like_mut)
        {
            let moved = renamed
                .iter()
                .filter_map(|(old, new)| profiles.remove(old).map(|item| (old, new, item)))
                .collect::<Vec<_>>();
            let moved_values = moved
                .iter()
                .filter_map(|(old, new, _)| {
                    previous
                        .profiles
                        .remove(*old)
                        .map(|profile| ((*new).clone(), profile))
                })
                .collect::<Vec<_>>();
            for (_, new, item) in moved {
                if profiles.contains_key(new) {
                    return Err(io::Error::other(format!(
                        "Profile {new} already exists in the configuration file"
                    )));
                }
                profiles.insert(new, item);
            }
            previous.profiles.extend(moved_values);
        }
        let previous = toml::Table::try_from(&previous).map_err(io::Error::other)?;
        let current = toml::Table::try_from(self).map_err(io::Error::other)?;
        let rendered = toml::to_string(self)
            .map_err(io::Error::other)?
            .parse::<toml_edit::DocumentMut>()
            .map_err(io::Error::other)?;
        merge_table(
            document.as_table_mut(),
            rendered.as_table(),
            &previous,
            &current,
        );
        let content = document.to_string();
        toml::from_str::<Self>(&content).map_err(io::Error::other)?;
        Ok(content)
    }
}

fn merge_table(
    document: &mut dyn toml_edit::TableLike,
    rendered: &dyn toml_edit::TableLike,
    previous: &toml::Table,
    current: &toml::Table,
) {
    for key in previous.keys() {
        if !current.contains_key(key) {
            document.remove(key);
        }
    }
    for (key, value) in current {
        let old = previous.get(key);
        if old == Some(value) {
            continue;
        }
        let item = rendered.get(key).expect("serialized config field");
        if let Some(existing) = document.get_mut(key) {
            if let (Some(table), Some(rendered), Some(value)) = (
                existing.as_table_like_mut(),
                item.as_table_like(),
                value.as_table(),
            ) {
                let empty = toml::Table::new();
                merge_table(
                    table,
                    rendered,
                    old.and_then(toml::Value::as_table).unwrap_or(&empty),
                    value,
                );
            } else {
                let mut replacement = item.clone();
                if let (Some(value), Some(original)) =
                    (replacement.as_value_mut(), existing.as_value())
                {
                    *value.decor_mut() = original.decor().clone();
                }
                *existing = replacement;
            }
        } else {
            document.insert(key, item.clone());
        }
    }
}

pub(crate) fn write_settings(path: &Path, contents: &str, mode: u32) -> io::Result<()> {
    static NEXT_FILE: AtomicU64 = AtomicU64::new(0);
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::other("invalid config path"))?;
    fs::create_dir_all(parent)?;
    let (temp, mut file) = loop {
        let id = NEXT_FILE.fetch_add(1, Ordering::Relaxed);
        let temp = path.with_extension(format!("{}.{id}.tmp", std::process::id()));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
        {
            Ok(file) => break (temp, file),
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(err) => return Err(err),
        }
    };
    let result = file
        .write_all(contents.as_bytes())
        .and_then(|()| file.set_permissions(fs::Permissions::from_mode(mode)))
        .and_then(|()| file.sync_all());
    drop(file);
    let result = result.and_then(|()| fs::rename(&temp, path));
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn default_is_valid() {
        let mut config = Config::default();
        config.normalize();
        assert!((0.4..=1.0).contains(&config.opacity));
        assert!(config.font_size >= 7);
    }

    #[test]
    fn invalid_user_values_are_bounded() {
        let mut config = Config {
            opacity: f64::NAN,
            font_size: 99,
            scrollback_lines: -10,
            font_family: "  ".into(),
            ..Config::default()
        };
        config.normalize();
        assert_eq!(config.opacity, Config::default().opacity);
        assert_eq!(config.font_size, 32);
        assert_eq!(config.scrollback_lines, 100);
        assert_eq!(config.font_family, Config::default().font_family);
    }

    #[test]
    fn unknown_fields_are_ignored_for_future_compatibility() {
        let content = "opacity = 0.75\nfuture_option = true\n";
        let config: Config = toml::from_str(content).expect("config should parse");
        assert_eq!(config.opacity, 0.75);
        assert!(config.blur);
    }

    #[test]
    fn editing_appearance_preserves_manual_settings_and_comments() {
        let source =
            "# My settings\nfont_size = 12 # points\nopacity = 0.95\n\n[future]\nflag = true\n";
        let previous = Config::default();
        let draft = Config {
            font_size: 16,
            padding_x: 14,
            ..previous.clone()
        };
        let result = draft
            .update_document(&previous, source, &Default::default())
            .unwrap();
        assert!(result.starts_with("# My settings\nfont_size = 16 # points\nopacity = 0.95"));
        assert!(result.contains("[future]\nflag = true"));
        let saved: Config = toml::from_str(&result).unwrap();
        assert_eq!(saved.font_size, 16);
        assert_eq!(saved.padding_x, 14);
        assert_eq!(saved.opacity, 0.95);
        assert!(draft
            .update_document(&previous, "font_size = [", &Default::default())
            .is_err());
    }

    #[test]
    fn shortcut_edits_keep_profile_and_unrelated_override_comments() {
        let source = "[shortcuts]\ncopy = [\"<Primary><Shift>c\"] # clipboard\nfuture = [] # leave it\n[profiles.Projects]\ndirectory = \"~/Projects\" # workspace\n";
        let previous: Config = toml::from_str(source).unwrap();
        let mut edited = previous.clone();
        edited
            .shortcuts
            .insert("copy".into(), vec!["<Primary><Shift>y".into()]);
        let output = edited
            .update_document(&previous, source, &Default::default())
            .unwrap();
        assert!(output.contains("# clipboard"));
        assert!(output.contains("future = [] # leave it"));
        assert!(output.contains("directory = \"~/Projects\" # workspace"));
        assert_eq!(
            toml::from_str::<Config>(&output).unwrap().shortcuts["copy"],
            vec!["<Primary><Shift>y"]
        );
    }

    #[test]
    fn background_style_respects_capability_and_user_preference() {
        let mut config = Config::default();
        assert_eq!(config.background_style(true), (config.opacity, true));
        assert_eq!(config.background_style(false), (1.0, false));
        config.blur = false;
        assert_eq!(config.background_style(false), (config.opacity, false));
        assert_eq!(config.background_style(true), (config.opacity, false));
        config.blur = true;
        config.opacity = 1.0;
        assert_eq!(config.background_style(true), (1.0, false));
    }
}
