#![windows_subsystem = "windows"]

use fltk::app;
use fltk::app::{event_coords, event_x, event_x_root, event_y, event_y_root};
use fltk::draw::{draw_rect, set_draw_color};
use fltk::enums::{Color, ColorDepth, Event, Key};
use fltk::frame::Frame;
use fltk::image::RgbImage;
use std::sync::{Arc, Mutex};
use std::thread;

use fltk::prelude::{GroupExt, ImageExt, WidgetBase, WidgetExt, WindowExt};
use fltk::window::Window;
use image::{DynamicImage, GenericImageView, RgbaImage};

use inputbot::KeybdKey::{LAltKey, NKey};
use lazy_static::lazy_static;
use win_screenshot::capture::capture_display;
use winapi::um::winuser::{HWND_TOPMOST, SWP_NOMOVE, SWP_NOSIZE};

use crate::utils::get_win_info;

extern "C" {
    fn SetWindowPos(
        hWnd: *mut std::os::raw::c_void,
        hWndInsertAfter: *mut std::os::raw::c_void,
        X: i32,
        Y: i32,
        cx: i32,
        cy: i32,
        uFlags: u32,
    ) -> bool;
}

mod utils;

#[cfg(target_os = "windows")]
mod systray;

type Hwnd = *mut std::os::raw::c_void;

pub static mut WINDOW: Hwnd = std::ptr::null_mut();

#[derive(Debug, Clone, Copy)]
pub enum Message {
    HideWindow,
    Message,
}

lazy_static! {
    // static ref ACTIVE_WINDOWS: Mutex<Vec<Window>> = Mutex::new(Vec::new());
    static ref MSG_WINDOW: Arc<Mutex<Option<Window>>> = Arc::new(Mutex::new(None));
    static ref MSG_FRAME: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    static ref AREA_FRAME: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    static ref AREA_WINDOW: Arc<Mutex<Option<Window>>> = Arc::new(Mutex::new(None));
    static ref IMG_C: Arc<Mutex<Option<DynamicImage>>> = Arc::new(Mutex::new(None));
}
fn main() {
    let (w, h, w2, h2, proportion) = get_win_info();
    let app = app::App::default();
    app::set_screen_scale(0, proportion);
    let mut wind = Window::new(0, 0, 1, 1, "");

    wind.set_border(false);
    wind.end();
    wind.show();

    let mut msg_wind = Window::new(0, 0, w2, h2, "Cropped Image");
    let mut msg_frame = Frame::default_fill();
    msg_wind.add(&msg_frame);
    msg_wind.fullscreen(true);
    msg_wind.set_border(false);
    msg_wind.end();

    let mut area_win = Window::new(0, 0, 500, 500, "");
    let area_frame = Frame::new(0, 0, w2, h2, "");
    area_win.set_border(false);
    area_win.add(&area_frame);
    area_win.end();
    *AREA_FRAME.lock().unwrap() = Some(area_frame);
    *AREA_WINDOW.lock().unwrap() = Some(area_win);
    let mut start_x = 0;
    let mut start_y = 0;
    let mut end_x = 0;
    let mut end_y = 0;

    msg_frame.handle(move |f, ev| {
        let img_c = Arc::clone(&IMG_C);
        let msg_wind_clone = Arc::clone(&MSG_WINDOW);
        let area_wind_clone = Arc::clone(&AREA_WINDOW);
        let area_frame = Arc::clone(&AREA_FRAME);
        match ev {
            Event::KeyDown => {
                if app::event_key() == Key::Escape {
                    let mut msg_wind = msg_wind_clone.lock().unwrap();
                    if let Some(msg_wind) = msg_wind.as_mut() {
                        msg_wind.hide();
                    }
                    true
                } else {
                    false
                }
            }
            Event::Push => {
                let mut area_wind = area_wind_clone.lock().unwrap();
                if let Some(w) = area_wind.as_mut() {
                    if w.width() > 5 && w.height() > 5 {
                        w.show();
                    }
                }
                start_x = event_x();
                start_y = event_y();
                end_x = (start_x as f32 * proportion) as i32; // 添加这行代码
                end_y = (start_y as f32 * proportion) as i32; // 添加这行代码

                f.redraw();
                true
            }
            Event::Drag => {
                end_x = event_x();
                end_y = event_y();
                f.redraw();
                true
            }
            Event::Released => {
                let w = ((end_x - start_x) as f32 * proportion) as u32;
                let h = ((end_y - start_y) as f32 * proportion) as u32;
                if w <= 5 || h <= 5 {
                    return false;
                }
                let start_x_n = (start_x as f32 * proportion) as u32;
                let start_y_n = (start_y as f32 * proportion) as u32;
                let img_c_guard = img_c.lock().unwrap();
                let img_c = img_c_guard.as_ref();
                let cropped_img = img_c.unwrap().view(start_x_n, start_y_n, w, h).to_image();
                let cropped_rgb_image = RgbImage::new(
                    (&cropped_img.into_iter()).as_ref(),
                    w as i32,
                    h as i32,
                    ColorDepth::Rgba8,
                )
                    .unwrap();

                let mut new_win = Window::new(
                    start_x,
                    start_y,
                    cropped_rgb_image.width(),
                    cropped_rgb_image.height(),
                    "Cropped Image",
                );

                let mut msg_wind = msg_wind_clone.lock().unwrap();
                if let Some(msg_wind) = msg_wind.as_mut() {
                    msg_wind.hide()
                }
                new_win.set_border(false);
                let mut new_frame = Frame::default_fill();
                new_frame.set_image_scaled(Some(cropped_rgb_image));
                new_frame.draw(move |f| {
                    set_draw_color(Color::from_rgba_tuple((0, 2, 2, 75)));
                    draw_rect(f.x(), f.y(), f.width(), f.height());
                });
                new_win.add(&new_frame);
                new_win.end();

                let mut offset = (0, 0);

                let new_win_c = new_win.clone();
                new_win.handle(move |win, ev| {
                    let new_win_c = new_win_c.clone();
                    match ev {
                        Event::KeyDown => {
                            if app::event_key() == Key::BackSpace || app::event_key() == Key::Escape
                            {
                                Window::delete(new_win_c);
                                true
                            } else {
                                false
                            }
                        }
                        Event::Push => {
                            offset = (event_coords().0, event_coords().1);
                            true
                        }
                        Event::Drag => {
                            let new_x = event_x_root() - offset.0;
                            let new_y = event_y_root() - offset.1;
                            win.set_pos(new_x, new_y);
                            true
                        }
                        _ => false,
                    }
                });

                new_win.show();
                unsafe {
                    SetWindowPos(
                        new_win.raw_handle(),
                        HWND_TOPMOST as *mut std::os::raw::c_void,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE,
                    );
                }
                // ACTIVE_WINDOWS.lock().unwrap().push(new_win);

                start_x = 0;
                start_y = 0;
                end_x = 0;
                end_y = 0;
                let mut area_wind = area_wind_clone.lock().unwrap();
                if let Some(w) = area_wind.as_mut() {
                    w.hide();
                }
                f.redraw();
                true
            }

            _ => false,
        };

        f.draw(move |_| {
            let mut area_win = area_wind_clone.lock().unwrap();
            let mut area_frame = area_frame.lock().unwrap();
            set_draw_color(Color::Red); // 设置绘制颜色
            let thickness = 5;
            for i in 0..thickness {
                draw_rect(
                    start_x - i,
                    start_y - i,
                    (end_x - start_x) + 2 * i,
                    (end_y - start_y) + 2 * i,
                );
            }
            for _ in 0..thickness {
                draw_rect(0, 0, w, h);
            }
            if let Some(win) = area_win.as_mut() {
                // win.set_pos(start_x, start_y);
                // win.set_size(end_x - start_x, end_y - start_y);
                win.resize(start_x, start_y, end_x - start_x, end_y - start_y);
                if let Some(f) = area_frame.as_mut() {
                    f.set_pos(-start_x, -start_y);
                }
            }
        });
        f.redraw();
        true
    });
    *MSG_FRAME.lock().unwrap() = Some(msg_frame);
    *MSG_WINDOW.lock().unwrap() = Some(msg_wind);
    let (s, r) = app::channel::<Message>();
    thread::spawn(move || {
        LAltKey.bind(move || {
            let img_c = Arc::clone(&IMG_C);
            let msg_w = Arc::clone(&MSG_WINDOW);
            let msg_frame = Arc::clone(&MSG_FRAME);
            let area_frame = Arc::clone(&AREA_FRAME);
            if LAltKey.is_pressed() {
                NKey.bind(move || {
                    let mut img_c = img_c.lock().unwrap();
                    let mut msg_frame = msg_frame.lock().unwrap();
                    let mut area_frame = area_frame.lock().unwrap();
                    let mut msg_w = msg_w.lock().unwrap();
                    if let Some(win) = msg_w.as_mut() {
                        if win.visible() {
                            return;
                        }
                    }
                    let buf = capture_display().unwrap();
                    let screenshot_data = buf.pixels.to_vec();
                    let img = DynamicImage::ImageRgba8(
                        RgbaImage::from_raw(buf.width, buf.height, buf.pixels).unwrap(),
                    );
                    *img_c = None;
                    *img_c = Some(img);
                    let fltk_screenshot = RgbImage::new(
                        &screenshot_data,
                        buf.width as i32,
                        buf.height as i32,
                        ColorDepth::Rgba8,
                    )
                        .unwrap();
                    let grayscale_screenshot = fltk_screenshot.convert(ColorDepth::L8).unwrap();
                    if let Some(frame) = msg_frame.as_mut() {
                        frame.set_image_scaled(Some(grayscale_screenshot));
                    }
                    if let Some(f) = area_frame.as_mut() {
                        f.set_size(buf.width as i32, buf.height as i32);
                        f.set_image(Some(fltk_screenshot));
                    }
                    if let Some(win) = msg_w.as_mut() {
                        win.set_size(w2, h2);
                        win.fullscreen(true);
                        // win.make_current();
                    }
                    s.send(Message::Message);
                    NKey.unbind();
                })
            }
        });
        inputbot::handle_input_events();
    });
    #[cfg(target_os = "windows")]
    {
        let msg_wind_clone = Arc::clone(&MSG_WINDOW);
        unsafe {
            WINDOW = wind.raw_handle();
        }
        wind.set_callback(|w| {
            w.platform_hide();
        });
        use crate::systray::NativeUi;
        systray::init().expect("Failed to init Native Windows GUI");
        let _ui = systray::SystemTray::build_ui(Default::default()).expect("Failed to build UI");
        systray::dispatch_thread_events_with_callback(move || {
            if wind.shown() {
                while app.wait() {
                    if let Some(msg) = r.recv() {
                        let mut msg_wind = msg_wind_clone.lock().unwrap();
                        match msg {
                            Message::Message => {
                                if let Some(msg_wind) = msg_wind.as_mut() {
                                    msg_wind.show()
                                }
                            }
                            Message::HideWindow => {}
                        }
                    }
                }
            } else {
                app::sleep(0.030);
            }
        });
    }
    app.run().unwrap();
}
