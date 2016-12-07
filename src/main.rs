#![feature(inclusive_range_syntax)]

extern crate gtk;
extern crate gtk_sys;
extern crate gdk;
extern crate glib;
extern crate cairo;

use gtk::prelude::*;
use glib::translate::*;

const WINDOW_DEFAULT_WIDTH: i32 = 80;
const WINDOW_DEFAULT_HEIGHT: i32 = 80;
const DEFAULT_DPI: i32 = 96;

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
    ($($n:ident),+ => move |$($p:tt : $z:ty),+| $body:expr) => (
        {
            $( let $n = $n.clone(); )+
            move |$(clone!(@param $p) : $z,)+| $body
        }
    );
    ($($n:ident),+ => move || -> $r:ty $body:block) => (
        {
            $( let $n = $n.clone(); )+
            move || -> $r $body
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

fn new_window_surface(window_size: (i32, i32), dpi_scale: f64) -> cairo::ImageSurface {
    let (window_width, window_height) = window_size;
    let window_y_center = window_height as f64 / 2.0;
    let window_x_center = window_width as f64 / 2.0;
    let circle_border = 5.0 * dpi_scale;
    let shadw_sigma = 2.0 * dpi_scale;
    let shadow_offset_x: f64 = 1.0 * dpi_scale;
    let shadow_offset_y: f64 = 1.0 * dpi_scale;
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

fn calculate_icons_position(x_center: f64,
                            y_center: f64,
                            radius: f64,
                            icons_num: i32)
                            -> Vec<(f64, f64)> {
    let mut result = Vec::with_capacity(icons_num as usize);

    let angle = 360.0_f64 / icons_num as f64;

    for i in 0..icons_num {
        let radians = (angle * i as f64).to_radians();
        let x = (radians.cos() * radius) + x_center;
        let y = (radians.sin() * radius) + y_center;

        result.push((x, y));
    }

    result
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }
    let dpi_scale = std::rc::Rc::new(std::cell::Cell::new(1.0));

    let window = gtk::Window::new(gtk::WindowType::Toplevel);
    window.set_title("Dropzone");
    window.set_default_size(WINDOW_DEFAULT_WIDTH, WINDOW_DEFAULT_WIDTH);
    window.set_app_paintable(true);
    window.set_position(gtk::WindowPosition::Center);
    window.set_type_hint(gdk::WindowTypeHint::Dock);

    let old_window_size = std::cell::Cell::new(None::<(i32, i32)>);
    let old_window_surface = std::cell::RefCell::new(None::<cairo::ImageSurface>);
    window.connect_draw(clone!(dpi_scale => move |window, cr| {
        cr.set_operator(cairo::Operator::Source);

        let window_size = window.get_size();
        if old_window_size.get().is_some() && window_size == old_window_size.get().unwrap() {
            // no need to redraw
            cr.set_source_surface(&old_window_surface.borrow()
                                       .clone()
                                       .unwrap(),
                                   0.0,
                                   0.0);
            cr.paint();
            return Inhibit(false);
        }
        old_window_size.set(Some(window_size));
        let surface = new_window_surface(window_size, dpi_scale.get());
        *old_window_surface.borrow_mut() = Some(surface.clone());

        cr.set_source_surface(&surface, 0.0, 0.0);

        cr.paint();

        Inhibit(false)
    }));

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    let update_visual =
        clone!(dpi_scale => move |window: &gtk::Window, _old_screen: &Option<gdk::Screen>| {
            let screen = gtk::WindowExt::get_screen(window).unwrap();
            let visual = screen.get_rgba_visual();
            window.set_visual(visual.as_ref());

            dpi_scale.set(screen.get_resolution() as f64 / DEFAULT_DPI as f64);
            window.resize((WINDOW_DEFAULT_WIDTH as f64 * dpi_scale.get()).ceil() as i32,
                          (WINDOW_DEFAULT_HEIGHT as f64 * dpi_scale.get()).ceil() as i32);
    });
    update_visual(&window, &None);
    window.connect_screen_changed(update_visual);

    make_window_draggable(&window);

    let icons_box = gtk::Layout::new(None, None);
    unsafe {
        // FIXME: use wrapped API
        gtk_sys::gtk_drag_dest_set(icons_box.to_glib_none().0,
                                   gtk_sys::GtkDestDefaults::all(),
                                   std::ptr::null_mut(),
                                   0,
                                   gdk::DragAction::all());
    }
    icons_box.drag_dest_add_text_targets();
    let icons_num = 6_i32;
    let icons = std::rc::Rc::new(std::cell::RefCell::new(Vec::with_capacity(icons_num as usize)));
    for _ in 0..icons_num {
        let icon = gtk::DrawingArea::new();
        icon.connect_draw(move |icon, cr| {
            let (width, _height) = icon.get_size_request();
            let center = width as f64 / 2.0;
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.5);
            cr.arc(center, center, center, 0.0, 2.0 * std::f64::consts::PI);
            cr.fill();
            Inhibit(false)
        });

        unsafe {
            // FIXME: use wrapped API
            gtk_sys::gtk_drag_dest_set(icon.to_glib_none().0,
                                       gtk_sys::GtkDestDefaults::all(),
                                       std::ptr::null_mut(),
                                       0,
                                       gdk::DragAction::all());
        }
        icon.drag_dest_add_text_targets();
        icon.connect_drag_data_received(move |_self, _drag_context, _x, _y, data, _info, _time| {
            if let Some(text) = data.get_text() {
                println!("{}", text);
            }
        });

        icons_box.put(&icon, 0, 0);
        icon.hide();
        icons.borrow_mut().push(icon);
    }

    let mouse_drag_in = std::rc::Rc::new(std::cell::Cell::new(false));
    icons_box.connect_drag_motion(clone!(mouse_drag_in, window, icons, dpi_scale =>
                                         move |icons_box, _context, _x, _y, _time| {
        if !mouse_drag_in.get() {
            println!("ENTER!");
            mouse_drag_in.set(true);
            let mut animation_step = 10;
            let mut animation = {
                let window = window.clone();
                let icons_box = icons_box.clone();
                let icons = icons.clone();
                let dpi_scale = dpi_scale.clone();
                let mouse_drag_in = mouse_drag_in.clone();
                move || -> gtk::Continue {
                    let (window_width, window_height) = window.get_size();
                    for (i, &(x, y)) in calculate_icons_position(window_width as f64 / 2.0,
                                                                 window_height as f64 / 2.0,
                                                                 window_width as f64 / 3.0,
                                                                 icons_num).iter().enumerate() {
                        let icon = &icons.borrow()[i];
                        let icon_size_target = window_width / 3 + if animation_step > 3 {
                            (20.0 * dpi_scale.get()) as i32 // 惯性效果
                        } else {
                            0
                        };
                        let (width, _height) = icon.get_size_request();
                        let icon_size_now = if width > 0 { width } else { 0 };
                        let icon_size = icon_size_now + ((icon_size_target - icon_size_now) / animation_step);
                        let center = icon_size as f64 / 2.0;

                        icon.set_size_request(icon_size, icon_size);

                        icons_box.move_(icon, (x - center) as i32, (y - center) as i32);
                        icon.show();
                    }
                    animation_step -= 1;
                    if animation_step > 0 && mouse_drag_in.get() {
                        Continue(true)
                    } else {
                        Continue(false)
                    }
                }
            };

            animation();
            gtk::timeout_add(16, animation);
        }
        true
    }));
    icons_box.connect_drag_leave(clone!(mouse_drag_in, window, icons_box, icons, dpi_scale => move |_, _, i| {
        if i == 0 && mouse_drag_in.get() {
            println!("LEAVE!");
            mouse_drag_in.set(false);
            let mut animation_step = 10;
            let mut animation = {
                let window = window.clone();
                let icons_box = icons_box.clone();
                let icons = icons.clone();
                let mouse_drag_in = mouse_drag_in.clone();
                move || -> gtk::Continue {
                    let (window_width, window_height) = window.get_size();
                    for (i, &(x, y)) in calculate_icons_position(window_width as f64 / 2.0,
                                                                 window_height as f64 / 2.0,
                                                                 window_width as f64 / 3.0,
                                                                 icons_num).iter().enumerate() {
                        let icon = &icons.borrow()[i];
                        let icon_size_target = 0;
                        let (width, _height) = icon.get_size_request();
                        let icon_size_now = if width > 0 { width } else { 0 };
                        let icon_size = icon_size_now + ((icon_size_target - icon_size_now) / animation_step);
                        let center = icon_size as f64 / 2.0;

                        icon.set_size_request(icon_size, icon_size);

                        icons_box.move_(icon, (x - center) as i32, (y - center) as i32);
                        icon.show();
                    }
                    animation_step -= 1;
                    if animation_step > 0 && !mouse_drag_in.get() {
                        Continue(true)
                    } else {
                        Continue(false)
                    }
                }
            };

            animation();
            gtk::timeout_add(16, animation);

        }
    }));

    window.add(&icons_box);

    window.show_all();

    gtk::main();
}
