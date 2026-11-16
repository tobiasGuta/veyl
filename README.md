# Veyl

**Veyl** is an early-stage, Fedora-native terminal emulator with a visual direction inspired by **Windows Terminal on Windows 11**: compact integrated tabs, a dark Fluent-like surface, and subtle translucency.

> **Status:** v0.1 development foundation. See [verification status](docs/STATUS.md) for desktop coverage and remaining release gates.

## Quick start — Fedora 45

Install the native libraries and Rust toolchain:

~~~bash
sudo dnf install rust cargo gcc pkgconf-pkg-config gtk4-devel vte291-gtk4-devel
# Optional preferred font; fallback to another monospace font if unavailable:
sudo dnf install cascadia-mono-fonts
~~~

From the repository root:

~~~bash
cargo run
~~~

Veyl opens a GTK4 window and uses VTE4 to spawn the current user's shell using the Linux pseudo-terminal subsystem. It does **not** replace Bash, Fish, Zsh, or the system terminal. Keep your existing terminal installed while evaluating it.

If Veyl cannot start the shell, the affected tab shows the error and can be closed normally. If the application fails to compile, please include the exact `cargo build --locked` output in your report.

### Desktop launcher and icon

Build from the repository root and run the user-local installer.

~~~bash
cargo build --locked --release
python3 scripts/install-local.py install
~~~

Open **Veyl** from the application grid. The installer registers the absolute executable and icon paths, so the launcher works independently of your desktop session’s `PATH` and icon-theme lookup. To remove the executable, launcher, and icon while keeping your settings, run:

~~~bash
python3 scripts/install-local.py uninstall
~~~

## What v0.1 includes in source

- Rust + GTK4 application with custom dark header, integrated selectable tabs, new/close tab buttons and Wayland-friendly GTK window management.
- VTE terminal widget per tab, launching the `$SHELL` executable (or `/bin/bash` fallback), with regular shell environment inheritance.
- Windows Terminal-inspired styling in [assets/veyl.css](assets/veyl.css).
- Settings tab with Appearance, Color schemes, Profiles, Shell prompt, and Keyboard shortcuts. Veyl saves them in `$XDG_CONFIG_HOME/veyl/config.toml` or `~/.config/veyl/config.toml`.
- Configurable font, font size, foreground/background colors, and scrollback via TOML.
- Clipboard shortcuts and basic tab shortcuts.
- No telemetry, command-history collection, AI execution, or remote service integration.

### Keyboard shortcuts

| Shortcut | Intended action |
| --- | --- |
| Ctrl+Shift+T | New tab |
| Ctrl+Shift+W | Close the focused pane, or the tab when it has one pane |
| Ctrl+Shift+C | Copy selection |
| Ctrl+Shift+V | Paste clipboard |
| Ctrl+Tab | Select the next tab, wrapping at the end |
| Ctrl+Shift+Tab | Select the previous tab, wrapping at the start |
| Ctrl+Shift+F | Find in the focused terminal |
| Ctrl+Shift+D | Start a new shell in the current terminal's folder |
| Alt+Shift+Plus / Minus | Split right / down |
| Alt+Arrow | Focus a neighboring pane in a split tab |

Open **Command Palette…** from the arrow beside **+**, or press **Ctrl+Shift+P**. Search across Veyl actions and launch profiles, use the arrow keys to select a result, and press Enter to run it. Results show the shortcuts currently assigned in Settings.

Right-click a tab to rename it, choose or reset its color, move it, duplicate it, or close it. Drag a tab title onto the left or right half of another tab to reorder it. With keyboard focus on a tab, **Shift+F10** or the **Menu** key opens the same menu. Clear the name in the rename dialog to restore the default name. Tab names and colors apply for the lifetime of that tab.

**Find in terminal** searches the focused pane's visible text and scrollback. Toggle **Aa** for case matching, use the arrows or Enter to move through matches, and press **Esc** to return to the terminal. Its query stays with the pane until you close it.

Ctrl-click a web URL in terminal output to open it. Right-click the terminal for Copy, Paste, Find, and Open current folder. The menu also offers Open link and Copy link when you right-click a web link.

**Duplicate tab**, **Split right**, and **Split down** start a fresh shell using the focused pane's profile and local folder. Veyl uses the shell's directory report when available and can read a local shell's current directory on Linux. If the report points to a remote or missing folder, Veyl uses the original starting folder. The tab's **×** closes all its panes.

Open **Keyboard shortcuts…** from the arrow beside **+** to change or clear bindings. Veyl shows an error when two app actions use the same shortcut. The menu also lists launch profiles and split commands.

**Caution:** Closing a tab can terminate or disconnect interactive programs. Do not test with unsaved work.

## Settings

Choose **Settings…** from the arrow beside **+** to open the Settings tab.

- **Appearance** controls the font, text color, background tint, frosted background, opacity, cursor, padding, and tab progress. The sample terminal previews typography and colors. The default frosted background uses a **72%** dark tint over compositor blur, with fully opaque terminal text.
- **Color schemes** shows ANSI swatches for built-in and valid custom palettes. The selected scheme applies to terminals that inherit the default. Profiles can override it in TOML.
- **Profiles** selects which configured profile the **+** button opens. Define shell commands, starting directories, and per-profile schemes in TOML.
- **Shell prompt** provides presets and a multiline Bash prompt editor.
- **Keyboard shortcuts** edits the bindings for supported terminal actions.
- **Interaction** controls the multiline clipboard preview. Veyl asks before pasting text that contains a line break by default when using its Paste action.

**Save** applies the draft and closes the tab. **Discard changes** or the Settings tab's close button drops the draft. You can switch to a terminal tab and return to the draft later. **Reset to defaults** resets the current section in the draft.

Frosted mode uses GTK 4.24 and the Wayland `ext-background-effect-v1` protocol. Veyl uses a solid dark background when this path is unavailable. Turn **Frosted background** off to use plain translucency, or set opacity to **100%** for a solid background. See the [visual spec](docs/VISUAL_SPEC.md).

### Bash prompt

Under **Settings → Shell prompt**, start with a preset or enter your own Bash `PS1` value. For example, `╭─⟡─\u in \w\n╰─❯ ` uses the current username and working directory on the first line. Bash handles colors, expansion, and prompt escapes when it displays the prompt. Veyl’s text preview displays common escapes with sample values and never evaluates shell expressions. See the [Bash prompt syntax reference](https://www.gnu.org/software/bash/manual/html_node/Controlling-the-Prompt.html).

**Save** stores the prompt for new Bash tabs and panes in Veyl. Choose **Shell default** or clear the editor to restore the prompt your shell already defines. The setting applies to Bash profiles and takes effect when a new shell starts.

**Apply to .bashrc…** writes the current editor text to a marked block after confirmation, independently of **Save**. It affects new Bash sessions that read your `.bashrc`, including those in other terminal apps. Use **Remove Veyl prompt from .bashrc…** to remove only that block.

## Configuration

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
~~~

Add profiles and reusable color schemes to the same file. Put root settings before the table headings. Existing tab titles and colors are session settings. Profile edits apply to newly opened tabs after relaunch.

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

Create the file manually to override defaults. Saving an appearance change also creates the file. Veyl preserves comments and unrelated keys when saving. Manual edits take effect at launch. If Veyl cannot read or parse the config at launch, it shows the path and error and offers to open the file. You can continue with defaults, then fix the file and reopen Veyl. Use [the window CSS selectors](docs/VISUAL_SPEC.md#customizing-the-window-chrome) to customize colors and spacing in the tab strip and frame.

## For Codex contributors

**Start with [AGENTS.md](AGENTS.md)**, then [Codex handoff](docs/CODEX_HANDOFF.md), [Architecture](docs/ARCHITECTURE.md), [Visual spec](docs/VISUAL_SPEC.md), [Testing](docs/TESTING.md), [Security](docs/SECURITY.md), and [Roadmap](docs/ROADMAP.md). Explicit acceptance criteria and non-goals are documented so agents don't fabricate untested features.

## Project identity and attribution

Veyl is an independent Linux project, not affiliated with Microsoft. “Windows Terminal”, “Fluent”, “Acrylic”, and “Mica” refer to external design inspiration and platform features, not Microsoft code or bundled graphics. Do not import Microsoft branding, proprietary assets, or copyrighted screenshots into the app.

No license has been selected by the repository owner yet. Do not infer permission to relicense the project; resolve that decision before publishing binary releases.
