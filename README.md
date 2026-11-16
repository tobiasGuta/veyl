# Veyl

**Veyl** is an early-stage, Fedora-native terminal emulator with a visual direction inspired by **Windows Terminal on Windows 11**: compact integrated tabs, a dark Fluent-like surface, and subtle translucency.

> **Status** · Fedora 45 GNOME Wayland is the tested platform. See [verification status](docs/STATUS.md) and [preview notes](docs/releases/v0.1.0-preview.md) for the scope of testing and the remaining release checks.

## Quick start — Fedora 45

Build and install Veyl for your account.

~~~bash
sudo dnf install git python3 rust cargo gcc pkgconf-pkg-config gtk4-devel vte291-gtk4-devel
git clone https://github.com/tobiasGuta/veyl.git
cd veyl
cargo build --locked --release
python3 scripts/install-local.py install
~~~

Open **Veyl** from the application grid. The installer registers the executable, desktop entry, and icon with absolute paths, so launching from GNOME does not depend on the desktop session’s `PATH`. The default font is Cascadia Mono. Install the optional `cascadia-mono-fonts` package or pick another monospace font in Settings.

To build and launch from the checkout without installing, run `cargo run --locked`. After updating an installation, reopen Veyl to use the new executable.

To remove the executable, desktop entry, and icon while keeping your settings, run:

~~~bash
python3 scripts/install-local.py uninstall
~~~

## Features

- Rust + GTK4 application with custom dark header, integrated selectable tabs, new/close tab buttons and Wayland-friendly GTK window management.
- One VTE terminal per pane, with the selected profile’s shell or your usual shell and environment.
- Windows Terminal-inspired styling in [assets/veyl.css](assets/veyl.css).
- Settings tab with Appearance, Color schemes, Profiles, Shell prompt, Keyboard shortcuts, and Interaction. Veyl saves them in `$XDG_CONFIG_HOME/veyl/config.toml` or `~/.config/veyl/config.toml`.
- Configurable font, font size, foreground/background colors, and scrollback via TOML.
- Split panes, scrollback search, tab actions, a command palette, and configurable shortcuts.
- No telemetry, command-history collection, AI execution, or remote service integration.

### Keyboard shortcuts

| Default shortcut | Action |
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
| Ctrl+Shift+Enter | Zoom or restore the focused split |
| Ctrl+= / Ctrl+- | Change the focused pane’s font size |
| Ctrl+0 | Reset the focused pane’s font size |
| Ctrl+Shift+P | Open the Command Palette |
| Ctrl+, | Open Settings |

Open **Command Palette…** from the arrow beside **+**, or press **Ctrl+Shift+P**. Search across Veyl actions and launch profiles, use the arrow keys to select a result, and press Enter to run it. Results show the shortcuts currently assigned in Settings.

Right-click a tab to rename it, choose or reset its color, move it, duplicate it, or close it. Drag a tab title onto the left or right half of another tab to reorder it. With keyboard focus on a tab, **Shift+F10** or the **Menu** key opens the same menu. Clear the name in the rename dialog to restore the automatic title. Enable **Show directory in tab title** under Appearance to use the active pane’s local directory when available. Directory changes update when the shell reports them. Tab names and colors apply for the lifetime of that tab.

**Find in terminal** searches the focused pane's visible text and scrollback. Toggle **Aa** for case matching, use the arrows or Enter to move through matches, and press **Esc** to return to the terminal. Its query stays with the pane until you close it.

Ctrl-click a web URL in terminal output to open it. Right-click the terminal for Copy, Paste, Find, and Open current folder. The menu also offers Open link and Copy link when you right-click a web link.

**Duplicate tab**, **Split right**, and **Split down** start a fresh shell using the focused pane's profile and local folder. Veyl uses the shell's directory report when available and can read a local shell's current directory on Linux. If the report points to a remote or missing folder, Veyl uses the original starting folder. The tab's **×** closes all its panes.

Open **Keyboard shortcuts…** from the arrow beside **+** to change or clear bindings. Veyl shows an error when two app actions use the same shortcut. The menu also lists launch profiles and split commands.

**Caution:** Closing a tab can terminate or disconnect interactive programs. Do not test with unsaved work.

## Settings

Choose **Settings…** from the arrow beside **+** to open the Settings tab.

- **Appearance** controls the font, text color, background tint, frosted background, opacity, cursor, padding, and tab progress. The sample terminal previews typography and colors. The default frosted background uses a **72%** dark tint over compositor blur, with fully opaque terminal text.
- **Color schemes** shows ANSI swatches for built-in and valid custom palettes. The selected scheme applies to terminals that inherit the default. Profiles can override it in TOML.
- **Profiles** creates, duplicates, renames, and edits launch profiles. Set the shell executable, starting directory, tab label, and color scheme, and choose which profile the **+** button opens.
- **Shell prompt** provides presets and a multiline Bash prompt editor.
- **Keyboard shortcuts** edits the bindings for supported terminal actions.
- **Interaction** controls the multiline clipboard preview. Veyl asks before pasting text that contains a line break by default when using its Paste action.

**Save** applies the draft and closes the tab. **Discard changes** or the Settings tab's close button drops the draft. You can switch to a terminal tab and return to the draft later. **Reset to defaults** resets the current section in the draft.

Frosted mode uses GTK 4.24 and the Wayland `ext-background-effect-v1` protocol. Veyl uses a solid dark background when this path is unavailable. Turn **Frosted background** off to use plain translucency, or set opacity to **100%** for a solid background. Very low opacity can make text hard to read over bright backgrounds. See the [visual spec](docs/VISUAL_SPEC.md).

### Bash prompt

Under **Settings → Shell prompt**, start with a preset or enter your own Bash `PS1` value. For example, `╭─⟡─\u in \w\n╰─❯ ` uses the current username and working directory on the first line. Bash handles colors, expansion, and prompt escapes when it displays the prompt. Veyl’s text preview displays common escapes with sample values and never evaluates shell expressions. See the [Bash prompt syntax reference](https://www.gnu.org/software/bash/manual/html_node/Controlling-the-Prompt.html).

**Save** stores the prompt for new Bash tabs and panes in Veyl. Choose **Shell default** or clear the editor to restore the prompt your shell already defines. The setting applies to Bash profiles and takes effect when a new shell starts.

Expand **Smart Bash prompt**, choose the segments you want, and select **Insert smart prompt** to put the colored prompt in the editable text field. It can show the full path, Git branch and status, Python virtual environment, exit status, command duration, SSH host, and a root marker. Timing requires Bash 5.3 or newer. Veyl loads the helper with the prompt for new Bash sessions.

**Apply to .bashrc…** writes the current editor text to a marked block after confirmation, independently of **Save**. It affects new Bash sessions that read your `.bashrc`, including those in other terminal apps. Use **Remove Veyl prompt from .bashrc…** to remove only that block.

## Configuration

The [configuration reference](docs/CONFIGURATION.md) covers TOML examples, profile fields, custom color schemes, shortcut overrides, and safe settings-file permissions. The [architecture](docs/ARCHITECTURE.md) and [test plan](docs/TESTING.md) cover development and verification.

## For Codex contributors

**Start with [AGENTS.md](AGENTS.md)**, then [Codex handoff](docs/CODEX_HANDOFF.md), [Architecture](docs/ARCHITECTURE.md), [Visual spec](docs/VISUAL_SPEC.md), [Testing](docs/TESTING.md), [Security](docs/SECURITY.md), and [Roadmap](docs/ROADMAP.md). Explicit acceptance criteria and non-goals are documented so agents don't fabricate untested features.

## Project identity and attribution

Veyl is an independent Linux project, not affiliated with Microsoft. “Windows Terminal”, “Fluent”, “Acrylic”, and “Mica” refer to external design inspiration and platform features, not Microsoft code or bundled graphics. Do not import Microsoft branding, proprietary assets, or copyrighted screenshots into the app.

Veyl is licensed under the [MIT License](LICENSE).
