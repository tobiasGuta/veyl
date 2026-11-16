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

## Session teardown

- Window actions and terminal callbacks hold weak references to the session state. Tab and window close cancel pending launches and release terminal widgets through VTE's disposal path.
- Shell launch errors are shown as text in the failed tab. Control characters from the error text are stripped before display.
- Tests observed Bash and an ordinary foreground `sleep` exit when their tab or window closed. Detached jobs and processes that ignore hangup can outlive the terminal. Session close does not guarantee termination of a process tree.

## Known v0.1 gaps to audit

- Settings reads reject non-regular files and files writable by other users. Writes check file ownership and directory permissions, then use an exclusive owner-only temporary file and atomic rename. Symlinked settings files are supported when the link and destination directories pass checks. Independent review of these changes and the session lifecycle remains open.
- Clipboard pastes may contain control sequences, newlines, and secrets; future safety prompts should not break existing terminal behavior.
- Translucent content can reveal desktop material behind the window; accessibility and shared-screen privacy need consideration.

## Reporting vulnerabilities

Until a security policy/contact is agreed with the owner, file a private disclosure where possible rather than posting exploitation details or secrets in public issues. Never include real terminal transcripts containing credentials in bug reports.
