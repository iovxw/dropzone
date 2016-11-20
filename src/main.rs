extern crate gtk;
extern crate gtk_sys;
extern crate gdk;
extern crate glib;

use gtk::prelude::*;
use gtk::{Label, Window, WindowType};

use glib::translate::*;

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = Window::new(WindowType::Toplevel);
    window.set_title("Dropzone");
    window.set_default_size(350, 70);
    window.set_keep_above(true);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_deletable(false);
    window.set_decorated(false);

    let zone = Label::new(Some("zone"));
    unsafe {
        // FIXME: use wrapped API
        gtk_sys::gtk_drag_dest_set(zone.to_glib_none().0,
                                   gtk_sys::GtkDestDefaults::all(),
                                   std::ptr::null_mut(),
                                   0,
                                   gdk::DragAction::all());
    }
    zone.drag_dest_add_text_targets();
    zone.connect_drag_data_received(|_self, _drag_context, _x, _y, data, _info, _time| {
        if let Some(text) = data.get_text() {
            println!("{}", text);
        }
    });

    window.add(&zone);
    window.show_all();

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}
