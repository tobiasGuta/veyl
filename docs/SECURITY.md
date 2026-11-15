# Security model

Veyl is a user-level terminal emulator, **not a sandbox**. Commands executed in the embedded shell run with the user's current privileges. Do not execute suspicious binaries merely because they are launched inside Veyl.

## Security boundaries

- The terminal app must not request root or polkit access for normal operation.
- VTE/GTK are dependencies; keep them current and use their intended APIs for PTY and escape sequence handling.
- Do not silently log terminal input, shell history, screen contents, clipboard contents or environment variables.
- Do not send user commands to AI/network services without opt-in and explicit preview/approval.
- Do not implement shell execution through `sh -c` for untrusted text or allow pasted text to auto-execute.
- Treat environment variables, terminal titles, escape sequences, OSC links and shell output as potentially attacker-controlled. Do not put them into shell command strings.
- Any hyperlinks should open only after user action and with scheme validation.
- Settings must be user-scoped; avoid insecure world-writable shared state, backups containing credentials, or plaintext session recording.
- In future SSH/workspace features, never embed private keys, tokens or passwords in repository files.

## Known v0.1 gaps to audit

- GTK signal handlers and strong reference cycles; use Weak references when required.
- Closing PTY surfaces can signal a process without proving all descendants exited. No blanket assurance against orphaned jobs.
- Settings writes are atomic rename but permissions/symlinks and temporary-file collisions need hardening.
- No UI error dialog when shell spawn fails; currently stderr only.
- Clipboard pastes may contain control sequences, newlines, and secrets; future safety prompts should not break existing terminal behavior.
- Translucent content can reveal desktop material behind the window; accessibility and shared-screen privacy need consideration.

## Reporting vulnerabilities

Until a security policy/contact is agreed with the owner, file a private disclosure where possible rather than posting exploitation details or secrets in public issues. Never include real terminal transcripts containing credentials in bug reports.
