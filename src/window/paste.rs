use gtk::prelude::*;
use gtk::{gio, glib};
use vte4::{prelude::*, Terminal};

pub(super) fn from_clipboard(terminal: &Terminal, confirm: bool) {
    if !confirm {
        terminal.paste_clipboard();
        return;
    }
    terminal.clipboard().read_text_async(
        gio::Cancellable::NONE,
        glib::clone!(
            #[weak]
            terminal,
            move |result| {
                if let Ok(Some(text)) = result {
                    if terminal.is_mapped() && terminal.is_input_enabled() {
                        paste_text(&terminal, text);
                    }
                }
            }
        ),
    );
}

pub(super) fn paste_text(terminal: &Terminal, text: glib::GString) {
    if !text.contains(['\n', '\r']) {
        terminal.paste_text(&text);
        return;
    }
    let Some(window) = terminal.root().and_downcast::<gtk::Window>() else {
        return;
    };
    let (preview, truncated) = preview(&text);
    let detail = if truncated {
        format!("First 500 characters of the clipboard:\n\n{preview}\n\nMore text follows.")
    } else {
        preview
    };
    gtk::AlertDialog::builder()
        .message("Paste text with line breaks?")
        .detail(detail)
        .buttons(["Cancel", "Paste"])
        .cancel_button(0)
        .default_button(0)
        .modal(true)
        .build()
        .choose(
            Some(&window),
            gio::Cancellable::NONE,
            glib::clone!(
                #[weak]
                terminal,
                move |result| {
                    if matches!(result, Ok(1))
                        && terminal.is_mapped()
                        && terminal.is_input_enabled()
                    {
                        terminal.paste_text(&text);
                    }
                }
            ),
        );
}

fn preview(text: &str) -> (String, bool) {
    let mut characters = text.chars();
    let preview = characters
        .by_ref()
        .take(500)
        .map(|ch| match ch {
            '\r' => "␍".to_owned(),
            '\n' => "↵\n".to_owned(),
            '\t' => "⇥".to_owned(),
            ch if ch.is_control() => format!("\\u{{{:04X}}}", ch as u32),
            ch => ch.to_string(),
        })
        .collect();
    (preview, characters.next().is_some())
}
