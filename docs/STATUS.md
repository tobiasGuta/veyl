# Verification status

Updated: 2026-11-15 (initial source publication)

**Classification:** Early v0.1 foundation, **not yet verified/released**. Documentation and source have been authored. A green Fedora desktop build and running GUI test have **not** been demonstrated in the authoring environment.

| Requirement | Status | Evidence / gap |
| --- | --- | --- |
| Repo initialized and source pushed | PENDING | Verify GitHub commit and tree on publication |
| Rust application source | WRITTEN | main.rs, window.rs, config.rs |
| Fedora 45 dependency package names | RESEARCHED | Fedora packages: gtk4-devel, vte291-gtk4-devel |
| Terminal PTY/shell launch | NOT RUN | Native runtime test required |
| Tab management | NOT RUN | Native runtime test plus lifecycle audit required |
| Clipboard hotkeys | NOT RUN | GUI test required |
| Background opacity | NOT RUN | Wayland compositor test required |
| True compositor blur | NOT IMPLEMENTED | Must not be claimed |
| CI | NOT RUN | Requires GitHub workflow execution |
| Cargo fmt/clippy/test/build locally | NOT RUN | No Rust/GTK4/VTE4 toolchain in current authoring container |
| Fedora 45 GNOME Wayland manual testing | NOT RUN | Access to a real desktop required |
| Security review | NOT RUN | Tab teardown and settings file handling need review |

Update this file **only with actual evidence**. Do not mark the project production-ready based solely on a code review, a mockup, or a passing headless build.
