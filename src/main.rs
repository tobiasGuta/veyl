mod config;
// Isolated protocol framing tests; no terminal graphics capability is enabled yet.
#[cfg(test)]
mod graphics;
#[cfg(feature = "graphics-prototype")]
mod graphics_preview;
#[cfg(test)]
mod graphics_transfer;
mod prompt;
mod window;

use gtk::prelude::*;
use gtk::{glib, Application};

#[cfg(not(feature = "graphics-prototype"))]
const APP_ID: &str = "io.github.tobiasGuta.Veyl";
#[cfg(feature = "graphics-prototype")]
const APP_ID: &str = "io.github.tobiasGuta.Veyl.GraphicsPrototype";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(window::build);
    app.run()
}
