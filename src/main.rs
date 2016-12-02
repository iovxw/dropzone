#![feature(inclusive_range_syntax)]

extern crate gtk;
extern crate gtk_sys;
extern crate gdk;
extern crate glib;
extern crate cairo;

use gtk::prelude::*;
use gdk::prelude::*;
use glib::translate::*;

// make moving clones into closures more convenient
macro_rules! clone {
    (@param _) => ( _ );
    (@param $x:ident) => ( $x );
    ($($n:ident),+ => move || $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move || $body
        }
    );
    ($($n:ident),+ => move |$($p:tt),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p),)+| $body
        }
    );
}

fn make_window_draggable(window: &gtk::Window) {
    let mouse_position = std::rc::Rc::new(std::cell::Cell::new((0.0, 0.0)));
    window.connect_button_press_event(clone!(mouse_position => move |_window, event| {
        let button = event.as_ref().button;
        if button == 1 {
            mouse_position.set(event.get_position());
        }
        Inhibit(false)
    }));
    window.connect_motion_notify_event(move |window, event| {
        if event.get_state().contains(gdk::ModifierType::from_bits(256).unwrap()) {
            let (mx, my) = mouse_position.get();
            let x = event.as_ref().x_root - mx;
            let y = event.as_ref().y_root - my;
            window.move_(x as i32, y as i32);
        }
        Inhibit(false)
    });
}

fn cairo_image_surface_blur_alpha(surface: &mut cairo::ImageSurface, sigma: f64) {
    let width = surface.get_width();
    let height = surface.get_height();

    let src_stride = surface.get_stride();
    let mut src = surface.get_data().unwrap();

    let ksize = (sigma * 3.0).ceil() as i32 * 2 + 1;
    if ksize == 1 {
        return;
    }

    let mut kernel: Vec<f64> = Vec::with_capacity(ksize as usize);
    let scale = -0.5 / (sigma * sigma);
    let cons = 1.0 / (-scale / std::f64::consts::PI).sqrt();

    let mut sum = 0.0;
    let kcenter = ksize / 2;
    for i in 0..ksize {
        let x = (i - kcenter) as f64;
        let n = cons * ((x * x * scale).exp());
        kernel.push(n);
        sum += n;
    }

    let kernel = kernel.iter().map(|n| n / sum).collect::<Vec<f64>>();

    for y in 0..height {
        for x in 0..width {
            sum = 0.0;
            let mut amul = 0.0;
            for i in -kcenter...kcenter {
                if (x + i) >= 0 && (x + i) < width {
                    amul += src[(y * src_stride + (x + i) * 4 + 3) as usize] as f64 *
                            kernel[(kcenter + i) as usize];
                }
                sum += kernel[(kcenter + i) as usize];
            }
            src[(y * src_stride + x * 4 + 3) as usize] = (amul / sum) as u8;
        }
    }

    for x in 0..width {
        for y in 0..height {
            sum = 0.0;
            let mut amul = 0.0;
            for i in -kcenter...kcenter {
                if (y + i) >= 0 && (y + i) < height {
                    amul += src[((y + i) * src_stride + x * 4 + 3) as usize] as f64 *
                            kernel[(kcenter + i) as usize];
                }
                sum += kernel[(kcenter + i) as usize];
            }
            src[(y * src_stride + x * 4 + 3) as usize] = (amul / sum) as u8;
        }
    }
}

fn new_window_surface(window_size: (i32, i32)) -> cairo::ImageSurface {
    let (window_width, window_height) = window_size;
    let window_y_center = window_height as f64 / 2.0;
    let window_x_center = window_width as f64 / 2.0;
    let circle_border = 5.0;
    let shadw_sigma = 2.0;
    let shadow_offset_x: f64 = 1.0;
    let shadow_offset_y: f64 = 1.0;
    let circle_radius = window_y_center - circle_border - shadow_offset_y.abs() -
                        (shadw_sigma * 3.0);

    let mut shadow =
        cairo::ImageSurface::create(cairo::Format::ARgb32, window_width, window_height);
    {
        let cr = cairo::Context::new(&shadow);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.arc(window_x_center + shadow_offset_x,
               window_y_center + shadow_offset_y,
               circle_radius + circle_border,
               0.0,
               2.0 * std::f64::consts::PI);
        cr.fill();
    }
    cairo_image_surface_blur_alpha(&mut shadow, shadw_sigma);
    let shadow_mask =
        cairo::ImageSurface::create(cairo::Format::ARgb32, window_width, window_height);
    {
        let cr = cairo::Context::new(&shadow_mask);
        cr.arc(window_x_center,
               window_y_center,
               circle_radius + circle_border,
               0.0,
               2.0 * std::f64::consts::PI);
        cr.fill();
        cr.set_operator(cairo::Operator::Out);
        cr.set_source_surface(&shadow, 0.0, 0.0);
        cr.paint();
    }
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, window_width, window_height);
    {
        let cr = cairo::Context::new(&surface);
        cr.set_source_rgba(0.8, 0.8, 0.8, 0.5);
        cr.arc(window_x_center,
               window_y_center,
               circle_radius + circle_border,
               0.0,
               2.0 * std::f64::consts::PI);
        cr.fill();
        cr.set_source_surface(&shadow_mask, 0.0, 0.0);
        cr.paint();
    }
    let mut shadow =
        cairo::ImageSurface::create(cairo::Format::ARgb32, window_width, window_height);
    {
        let cr = cairo::Context::new(&shadow);
        cr.set_source_rgba(0.0, 0.0, 0.0, 0.5);
        cr.arc(window_x_center + shadow_offset_x,
               window_y_center + shadow_offset_y,
               circle_radius,
               0.0,
               2.0 * std::f64::consts::PI);
        cr.fill();
    }
    cairo_image_surface_blur_alpha(&mut shadow, shadw_sigma);
    {
        let cr = cairo::Context::new(&surface);
        cr.set_source_surface(&shadow, 0.0, 0.0);
        cr.paint();
    }
    let mask = cairo::ImageSurface::create(cairo::Format::ARgb32, window_width, window_height);
    {
        let cr = cairo::Context::new(&mask);
        cr.arc(window_x_center,
               window_y_center,
               circle_radius,
               0.0,
               2.0 * std::f64::consts::PI);
        cr.fill();
        cr.set_operator(cairo::Operator::Out);
        cr.set_source_surface(&surface, 0.0, 0.0);
        cr.paint();
    }
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, window_width, window_height);
    {
        let cr = cairo::Context::new(&surface);
        cr.set_source_rgba(1.0, 1.0, 1.0, 0.9);
        cr.arc(window_x_center,
               window_y_center,
               circle_radius,
               0.0,
               2.0 * std::f64::consts::PI);
        cr.fill();
        cr.set_source_surface(&mask, 0.0, 0.0);
        cr.paint();
    }

    surface
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Dropzone");
    window.set_default_size(80, 80);
    window.set_app_paintable(true);
    window.set_position(gtk::WindowPosition::Center);
    window.set_type_hint(gdk::WindowTypeHint::Dock);

    let old_window_size = std::cell::Cell::new(None::<(i32, i32)>);
    let old_window_surface = std::cell::RefCell::new(None::<cairo::ImageSurface>);
    window.connect_draw(move |window, _context| {
        let wcr = cairo::Context::create_from_window(&window.get_window().unwrap());
        wcr.set_antialias(cairo::Antialias::None);
        // set window to transparent
        wcr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
        wcr.set_operator(cairo::Operator::Source);

        let window_size = window.get_size();
        if old_window_size.get().is_some() && window_size == old_window_size.get().unwrap() {
            // no need to redraw
            wcr.set_source_surface(&old_window_surface.borrow()
                                       .clone()
                                       .unwrap(),
                                   0.0,
                                   0.0);
            wcr.paint();
            return Inhibit(false);
        }
        old_window_size.set(Some(window_size));
        let surface = new_window_surface(window_size);
        *old_window_surface.borrow_mut() = Some(surface.clone());

        wcr.set_source_surface(&surface, 0.0, 0.0);

        wcr.paint();
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

    make_window_draggable(&window);

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
    zone.connect_drag_data_received(clone!(window =>
                                           move |_self, _drag_context, _x, _y, data, _info, _time| {
        if let Some(text) = data.get_text() {
            let (height,width) = window.get_size();
            // FIXME: just for test
            window.resize(height + 10, width + 10);
            println!("{}", text);
        }
    }));
    window.add(&zone);

    window.show_all();

    gtk::main();
}
