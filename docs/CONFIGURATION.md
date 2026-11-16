# Configuration reference

Veyl reads `$XDG_CONFIG_HOME/veyl/config.toml`. When `XDG_CONFIG_HOME` is unset or empty, the path is `~/.config/veyl/config.toml`. Open **Settings** with **Ctrl+,** for the primary controls. This guide covers hand-editing TOML and managing dotfiles.

## Examples

Example `~/.config/veyl/config.toml`:

~~~toml
opacity = 0.72
blur = true
font_family = "Cascadia Mono"
font_size = 12
scrollback_lines = 10000
background = "#10121a"
foreground = "#e8eaf0"
cursor_shape = "block"
padding_x = 10
padding_y = 8
confirm_multiline_paste = true
directory_tab_titles = false
~~~

Add profiles and reusable color schemes to the same file. Put root settings before the table headings. Existing tab titles and colors are session settings. Changes saved in Settings apply to new profile launches immediately. Running shells keep their executable and starting environment. After editing TOML by hand, reopen Veyl to load it.

~~~toml
default_profile = "Projects"
color_scheme = "Midnight"
tab_progress = true

[profiles.Bash]
shell = "/bin/bash"

[profiles.Projects]
shell = "/bin/bash"
directory = "~/Projects"
tab_name = "Project shell"
color_scheme = "Workshop"

[schemes.Workshop]
ansi = [
  "#161924", "#f17883", "#91d7a2", "#dfc27a",
  "#81aaff", "#c8a1ed", "#7dcedb", "#dae2f2",
  "#777f99", "#ff9da6", "#aae9b2", "#f5d995",
  "#a9c5ff", "#dfbaff", "#a3e2ef", "#ffffff",
]
cursor = "#b8cdff"
selection = "#3d5180"

[shortcuts]
find = ["<Primary><Shift>f"]
~~~

An omitted shell uses your login shell setting. Use an absolute executable path and an absolute directory or one starting with `~/`. A scheme's `ansi` array is ordered from black through white, then bright black through bright white. Set a shortcut list to `[]` to unbind it. Leave `default_profile` empty to launch your ordinary shell from **+**.

Create the file manually to override defaults. Saving an appearance change also creates the file. Veyl preserves comments and unrelated keys when saving. Manual edits take effect at launch. If Veyl cannot read or parse the config at launch, it shows the path and error and offers to open the file. You can continue with defaults, then fix the file and reopen Veyl. Use [the window CSS selectors](VISUAL_SPEC.md#customizing-the-window-chrome) to customize colors and spacing in the tab strip and frame.

## Settings file permissions

Veyl accepts regular settings files in directories controlled by your account or the system. It refuses files and directories with unsafe ownership or write permissions. Owner-controlled dotfile symlinks are supported when both the link location and its target pass the checks. Config saves use an atomic replacement and set the file mode to `0600`. The explicit **Apply to .bashrc…** action keeps the ordinary permissions of a permitted existing `.bashrc`.

If Veyl reports a permissions error, inspect the path and any symlink destination it names before changing ownership or permissions. See the [security model](SECURITY.md) for the trust boundaries.
