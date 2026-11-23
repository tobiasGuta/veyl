# Terminal graphics: Kitty / Codex CLI compatibility

Status: **research and isolated framing prototype; NOT supported at runtime**  
Target: Fedora 45 GNOME Wayland, Rust + GTK4 + VTE 0.84.1  
Started: 2026-11-22

## User-visible goal

Display a Codex CLI image preview directly inside a Veyl terminal pane, without switching to another terminal. Codex currently shows "Image preview unavailable in this terminal" in Veyl.

**This branch does not fix that message yet.** It must not be advertised as a working terminal graphics feature.

## Confirmed architecture constraints

Veyl uses upstream VTE4 for parsing PTY output and drawing text. The Veyl Rust application does not receive a copy of every byte written to the child PTY. VTE 0.84 does not implement the Kitty graphics protocol; a Rust escape-sequence parser bolted onto the application cannot, by itself, make VTE render images.

The presence of historical SIXEL properties alone does not establish working SIXEL support in Fedora's VTE build. Do not set TERM to xterm-kitty, inject KITTY_WINDOW_ID, advertise SIXEL DA1, or answer Kitty query actions before implementing actual graphics support.

References:

- Kitty graphics protocol: https://sw.kovidgoyal.net/kitty/graphics-protocol/
- VTE GTK4 API: https://gnome.pages.gitlab.gnome.org/vte/gtk4/
- Fedora 45 VTE: https://packages.fedoraproject.org/pkgs/vte291/vte291-gtk4/fedora-45.html
- Codex protocol selection: https://github.com/openai/codex/blob/main/codex-rs/tui/src/pets/image_protocol.rs
- Codex terminal probe: https://github.com/openai/codex/blob/main/codex-rs/tui/src/terminal_probe.rs

## What this branch actually implements

The isolated pure Rust frame splitter in src/graphics.rs:

- Recognizes Kitty APC frames (ESC _ G ... ESC backslash) across arbitrary reads.
- Preserves normal PTY data and foreign APC sequences in order.
- Bounds buffered image frames; rejects oversize frames.
- Drops unfinished graphics on EOF rather than leaking payload into terminal text.
- Tests normal output, fragmentation, unrelated sequences, oversize, and EOF.

This component only separates transport frames. It does NOT decode images, assemble Kitty chunks, store or place or draw images, answer graphics capability queries, or intercept a live PTY. It is compiled under cfg(test) only, with no effect on the installed Veyl application.

## Proposed implementation phases

1. **PTY/backend proof:** Find an upstream VTE graphics path or validate an opt-in PTY bridge that forwards shell I/O to VTE unchanged while extracting Kitty frames. Replacing the existing default PTY launch path is out of scope until session safety is proven.
2. **Real GTK preview:** Build a one-image overlay fixture positioned relative to VTE terminal cells. Verify clipping to pane bounds, scrolling, resize, fractional scaling, and alternate screen.
3. **Image protocol:** Implement bounded base64/chunk assembly, image data types, IDs, placement IDs, deletion, acknowledgments and resource cleanup. Reject unsupported actions truthfully.
4. **Codex integration:** Validate graphics protocol query/replies against the installed Codex version. Codex may prefer a Kitty local-file transfer variant; define a strict local-file trust and access policy. Do not spoof another terminal's brand.
5. **End-to-end regression:** Validate a real attached or viewed PNG inside the Codex preview in Veyl under Fedora Wayland, across splits and tab closure. Preserve normal shells, vim, tmux, VTE output, cursor/mouse behavior, OSC links, copy/paste, accessibility, signals, foreground job cleanup, and process lifetime.

## Threat model and constraints

- Do not enable image support based on TERM alone.
- Bound compressed bytes, decompressed pixels, image count, incomplete transfers, and CPU decode work before allocating textures.
- Never auto-fetch URLs or blindly open file paths supplied by terminal output. Do not weaken Veyl's existing file-URI restrictions.
- Clear images and unfinished transmissions when closing panes, tabs and sessions.
- Do not grant image decoding any additional privileges.
- Do not change the main branch, installed Veyl or the default VTE execution path until this feature is actually validated.

## Test status

The parser tests are included in the normal cargo test --locked invocation.

Run the standard quality gates on Fedora 45:

~~~bash
cargo fmt -- --check
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked
~~~

Manual terminal graphics tests: **NOT RUN**. Full image support is **NOT IMPLEMENTED** in this branch.

A future graphics implementation is only complete after Codex's original full-color PNG preview renders directly inside Veyl and the existing Fedora Wayland tests pass.
