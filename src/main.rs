mod config;
mod window;

use gtk::prelude::*;
use gtk::{glib, Application};

const APP_ID: &str = "io.github.tobiasGuta.Veyl";

fn main() -> glib::ExitCode {
    let app = Application::builder().application_id(APP_ID).build();
    app.connect_activate(window::build);
    app.run()
}
