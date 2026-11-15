use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub opacity: f64,
    pub font_family: String,
    pub font_size: u16,
    pub scrollback_lines: i64,
    pub background: String,
    pub foreground: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            opacity: 0.86,
            font_family: "Cascadia Mono".to_owned(),
            font_size: 12,
            scrollback_lines: 10_000,
            background: "#10121a".to_owned(),
            foreground: "#e8eaf0".to_owned(),
        }
    }
}

impl Config {
    pub fn normalize(&mut self) {
        if !self.opacity.is_finite() {
            self.opacity = Self::default().opacity;
        }
        self.opacity = self.opacity.clamp(0.40, 1.0);
        self.font_size = self.font_size.clamp(7, 32);
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

    pub fn load() -> Self {
        let Some(path) = Self::path() else {
            eprintln!("Veyl: no HOME or XDG_CONFIG_HOME; using defaults");
            return Self::default();
        };
        let mut config = match fs::read_to_string(&path) {
            Ok(contents) => match toml::from_str::<Self>(&contents) {
                Ok(value) => value,
                Err(err) => {
                    eprintln!("Veyl: invalid {}: {err}; using defaults", path.display());
                    Self::default()
                }
            },
            Err(err) if err.kind() == io::ErrorKind::NotFound => Self::default(),
            Err(err) => {
                eprintln!("Veyl: unable to read {}: {err}; using defaults", path.display());
                Self::default()
            }
        };
        config.normalize();
        config
    }

    pub fn save(&self) -> io::Result<()> {
        let path = Self::path().ok_or_else(|| io::Error::other("no user config directory"))?;
        let parent = path.parent().ok_or_else(|| io::Error::other("invalid config path"))?;
        fs::create_dir_all(parent)?;
        let content = toml::to_string_pretty(self).map_err(io::Error::other)?;
        // Do not log contents or terminal output. Atomically replace settings on save.
        let temp = path.with_extension(format!("{}.tmp", std::process::id()));
        fs::write(&temp, content)?;
        if let Err(err) = fs::rename(&temp, &path) {
            let _ = fs::remove_file(&temp);
            return Err(err);
        }
        Ok(())
    }
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
    }
}
