extern crate gtk;
extern crate gtk_sys;
extern crate gdk;
extern crate glib;
extern crate cairo;

use gtk::prelude::*;
use gdk::prelude::*;
use glib::translate::*;

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Dropzone");
    window.set_default_size(64, 64);
    window.set_keep_above(true);
    window.set_skip_taskbar_hint(true);
    window.set_skip_pager_hint(true);
    window.set_deletable(false);
    window.set_decorated(false);
    window.set_app_paintable(true);

    window.connect_draw(|window, _context| {
        let cr = cairo::Context::create_from_window(&window.get_window().unwrap());
        // set window to transparent
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        cr.set_operator(cairo::Operator::Source);
        cr.paint();

        // draw a circle
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.arc(32.0, 32.0, 32.0, 0.0, 2.0 * std::f64::consts::PI);
        cr.fill();
        Inhibit(false)
    });
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    let update_visual = |window: &gtk::Window, _old_screen: &Option<gdk::Screen>| {
        let screen = gtk::WindowExt::get_screen(window).unwrap();
        let visual = screen.get_rgba_visual();

        window.set_visual(visual.as_ref());
    };
    update_visual(&window, &None);
    window.connect_screen_changed(update_visual);

    let zone = gtk::Label::new(Some("zone"));
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

    gtk::main();
}
