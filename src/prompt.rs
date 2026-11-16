use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const BEGIN: &str = "# BEGIN VEYL BASH PROMPT";
const END: &str = "# END VEYL BASH PROMPT";
const HOOK: &str = r#"if ! declare -F __veyl_set_prompt >/dev/null; then
    __veyl_set_prompt() { PS1=$__veyl_prompt; return "$1"; }
    PROMPT_COMMAND+=('__veyl_set_prompt "$?"')
fi
"#;

pub fn startup_file() -> io::Result<PathBuf> {
    let path = crate::config::Config::path()
        .ok_or_else(|| io::Error::other("no user config directory"))?
        .with_file_name("bash-prompt-init.bash");
    let script = format!(
        r#"__veyl_local_prompt=$VEYL_BASH_PROMPT
unset VEYL_BASH_PROMPT
if [[ -r ~/.bashrc ]]; then
    source ~/.bashrc
fi
__veyl_prompt=$__veyl_local_prompt
unset __veyl_local_prompt
{HOOK}"#
    );
    crate::config::write_settings(&path, &script, 0o600)?;
    Ok(path)
}

pub fn bashrc() -> io::Result<PathBuf> {
    std::env::var_os("HOME")
        .filter(|home| !home.is_empty())
        .map(|home| PathBuf::from(home).join(".bashrc"))
        .ok_or_else(|| io::Error::other("HOME is unavailable"))
}

pub fn update_bashrc(path: &Path, prompt: Option<&str>) -> io::Result<()> {
    let (path, mode) = match fs::symlink_metadata(path) {
        Ok(_) => {
            let path = fs::canonicalize(path)?;
            let metadata = fs::metadata(&path)?;
            if !metadata.is_file() {
                return Err(io::Error::other(".bashrc must be a regular file"));
            }
            (path, metadata.permissions().mode() & 0o777)
        }
        Err(err) if err.kind() == io::ErrorKind::NotFound => (path.to_path_buf(), 0o600),
        Err(err) => return Err(err),
    };
    let original = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) if err.kind() == io::ErrorKind::NotFound => String::new(),
        Err(err) => return Err(err),
    };
    let updated = edit_bashrc(&original, prompt)?;
    if updated != original {
        crate::config::write_settings(&path, &updated, mode)?;
    }
    Ok(())
}

fn edit_bashrc(original: &str, prompt: Option<&str>) -> io::Result<String> {
    let mut markers = original
        .split_inclusive('\n')
        .scan(0, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, *offset, line.trim_end_matches('\n')))
        })
        .filter(|(_, _, line)| matches!(*line, BEGIN | END));

    let (range, at_end) = match (markers.next(), markers.next(), markers.next()) {
        (None, None, None) => (original.len()..original.len(), true),
        (Some((start, _, BEGIN)), Some((_, end, END)), None) => {
            // Include the newline Veyl prepended when it created the block.
            let at_end = end == original.len();
            (start.saturating_sub(usize::from(at_end))..end, at_end)
        }
        _ => {
            return Err(io::Error::other(
                "Veyl's .bashrc prompt markers are incomplete or duplicated",
            ))
        }
    };
    let mut result = original.to_owned();
    let block = match prompt {
        Some(value) => {
            validate(value)?;
            let mut quoted = String::new();
            for ch in value.chars() {
                match ch {
                    '\\' => quoted.push_str(r"\\"),
                    '\'' => quoted.push_str(r"\'"),
                    '\n' => quoted.push_str(r"\n"),
                    '\r' => quoted.push_str(r"\r"),
                    _ => quoted.push(ch),
                }
            }
            format!(
                "\n{BEGIN}\nif [[ $- == *i* ]]; then\n__veyl_prompt=$'{quoted}'\n{HOOK}fi\n{END}\n"
            )
        }
        None => String::new(),
    };
    let block = if at_end {
        &block
    } else {
        block.strip_prefix('\n').unwrap_or(&block)
    };
    result.replace_range(range, block);
    Ok(result)
}

pub fn validate(prompt: &str) -> io::Result<()> {
    if prompt.len() > 8192 || prompt.contains('\0') {
        return Err(io::Error::other(
            "Enter a Bash prompt of at most 8 KiB, without NUL characters",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bashrc_updates_only_the_managed_block() {
        for original in ["", "# user settings", "# user settings\n"] {
            let prompt = r"\u:\w \' $ ";
            let installed = edit_bashrc(original, Some(prompt)).unwrap();
            assert!(installed.starts_with(original));
            assert_eq!(edit_bashrc(&installed, Some(prompt)).unwrap(), installed);
            let updated = edit_bashrc(&installed, Some("custom\n❯ ")).unwrap();
            assert!(updated.contains(r"custom\n❯ "));
            assert_eq!(edit_bashrc(&updated, None).unwrap(), original);
        }
        let installed = edit_bashrc("# existing", Some("❯ ")).unwrap();
        assert_eq!(
            edit_bashrc(&(installed + "# appended"), None).unwrap(),
            "# existing\n# appended"
        );
        assert!(edit_bashrc(BEGIN, None).is_err());
    }
}
