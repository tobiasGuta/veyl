mod config;
// Isolated protocol framing tests; no terminal graphics capability is enabled yet.
#[cfg(test)]
mod graphics;
mod prompt;
mod window;

use gtk::prelude::*;
use gtk::{glib, Application};

const APP_ID: &str = "io.github.tobiasGuta.Veyl";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(window::build);
    app.run()
}
