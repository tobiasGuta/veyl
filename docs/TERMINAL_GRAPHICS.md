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


## Confirmed user environment and exact Codex behavior (2026-11-22)

User confirmed the following in Veyl, on Fedora 45:

~~~text
codex-cli 0.171.0
vte291-gtk4-0.84.1-1.fc45.x86_64
TERM=xterm-256color
COLORTERM=truecolor
~~~

Codex **0.171.0** source tag `rust-v0.171.0` implements the following in `codex-rs/tui/src/pets/image_protocol.rs`:

- Auto graphics detection uses known Kitty/WezTerm/Ghostty/Rex environments or SIXEL terminal identifiers; `TERM=xterm-256color` and `COLORTERM=truecolor` do not imply image support.
- PNG inline data uses Kitty graphics APC actions `ESC _ G ... ESC backslash` with first command `a=T,t=d,f=100,c=<columns>,r=<rows>,q=2,i=<image_id>,m=<0-or-1>,C=1`; each base64 payload chunk is at most 4,096 bytes.
- Continuation commands use `m=<0-or-1>` until the last chunk. Handling *one APC frame* is not the same as decoding *one complete PNG transfer*.
- The delete command uses `a=d,d=I,i=<id>,q=2`; movement can delete placements with lowercase `d=i` then place again with `a=p,i=<id>,c=<columns>,r=<rows>,C=1,q=2`.
- iTerm2 may get a Kitty local-file variant, but Veyl should prioritize byte-stream `t=d` with no file reads.
- An explicit Codex image-protocol override may bypass automatic detection, **but must never be used for Veyl until rendering is real**. Misrepresenting image capabilities just exposes unrendered escape sequences.

Tag source: https://github.com/openai/codex/blob/rust-v0.171.0/codex-rs/tui/src/pets/image_protocol.rs

Next acceptance test input: a known PNG emitted as the exact above chunk series (with interleaved cursor moves and deletions), followed by a real Codex image preview inside a split pane. This is a **test target**, not a claim that the UI works.


## Phase 2: opt-in GTK drawing proof of concept

A GTK4 Picture is now attached to the existing pane-local Search overlay in an **experimental build only**, guarded by the cargo feature named graphics-prototype. This is a rendering fixture: it loads a local PNG chosen by the developer through VEYL_GRAPHICS_PREVIEW_FILE and draws it **inside the Veyl pane surface**. The picture is non-interactive, so mouse input passes through to VTE. The overlay belongs to the pane and disappears when the pane closes.

The graphics-prototype build uses a distinct GTK application ID, so it does not interfere with the running installed Veyl application. It does not change the normal build.

To perform a physical Wayland rendering test on Fedora 45, from a checked-out feature branch and with a PNG you control:

~~~bash
VEYL_GRAPHICS_PREVIEW_FILE="$HOME/Pictures/test.png" \
  cargo run --locked --features graphics-prototype
~~~

Replace the file path with an existing PNG. The program must show the image over the terminal area in the new experimental Veyl window. Check that shell input still works when clicking through the preview, tabs and splits continue to work, and the image disappears when its pane closes. This is a *static position* prototype; it deliberately does not follow terminal cursor movement or scrollback.

Development restrictions:

- File read limit: 8 MiB, including a length cap on reading to handle concurrent file changes.
- PNG IHDR dimension guard: max 16,384 per dimension and 16 million pixels; GDK still validates full encoded PNG data.
- No network fetching, image file auto-discovery, or terminal-output-triggered file access.
- The feature does not cause Codex to send image previews: PTY capture, Kitty transfer parsing integration, per-cell positioning, scrolling, image lifecycle, and terminal capability detection remain TODO.
- Do not install or merge the experimental feature in lieu of genuine Codex graphics support.

CI now tests, formats, lints and builds both the default Veyl binary and its graphics-prototype feature. **CI is not a substitute for a physical Wayland visual test.**


## Codex multi-chunk PNG transport tests

The test-only module src/graphics_transfer.rs now models the Codex 0.171.0 PNG chunk stream more closely than the raw frame splitter. It validates the initial action and transport (a=T,t=d,f=100), preserves the image ID and requested cell dimensions, buffers continuation frames marked m=1, decodes after m=0, and rejects malformed or oversized transfers. The decoder has a compressed-size bound of 8 MiB, an encoded-size bound of 12 MiB, and a 16-million-pixel PNG IHDR bound. A complete PNG decoder still has to validate the full image data.

This transfer assembler **is not connected to any PTY or live Codex process**. Deletion, placement, alternate-screen cursor mapping, and image lifecycle are still outstanding. It runs only under cargo test and cannot change normal terminal operation. Static GTK drawing and transfer assembly are separate milestones to connect after the PTY architecture is safe.
