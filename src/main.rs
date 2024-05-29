#![windows_subsystem = "windows"]

use fltk::app::{event_x, event_y};
use fltk::draw::{draw_rect, set_draw_color};
use fltk::enums::{Color, ColorDepth, Event, Key};
use fltk::frame::Frame;
use fltk::image::RgbImage;
use fltk::{app, window};
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
    static ref DRAG_WINDOW: Arc<Mutex<Option<Window>>> = Arc::new(Mutex::new(None));
    static ref AREA_FRAME: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
    static ref AREA_WINDOW: Arc<Mutex<Option<Window>>> = Arc::new(Mutex::new(None));
    // static ref DRAG_FRAME: Arc<Mutex<Option<Frame>>> = Arc::new(Mutex::new(None));
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
    let mut drag_wind = Window::new(0, 0, 500, 500, "Cropped Image");
    drag_wind.set_border(false);

    // let mut drag_frame = Frame::default_fill();
    // drag_frame.draw(move |f| {
    //     // Draw the border
    //     set_draw_color(Color::Red); // Set the border color
    //     let border_width = 5;
    //     for i in 0..border_width {
    //         draw_rect(i, i, f.w() - 2 * i, f.h() - 2 * i);
    //     }
    // });
    // drag_wind.add(&drag_frame);
    drag_wind.end();
    // *DRAG_FRAME.lock().unwrap() = Some(drag_frame);
    *DRAG_WINDOW.lock().unwrap() = Some(drag_wind);
    let mut area_win = Window::new(0, 0, 500, 500, "");
    let mut area_frame = Frame::new(0, 0, w2, h2, "");
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
        let drag_wind_clone = Arc::clone(&DRAG_WINDOW);
        let area_wind_clone = Arc::clone(&AREA_WINDOW);
        let area_frame = Arc::clone(&AREA_FRAME);

        // let drag_frame_clone = Arc::clone(&DRAG_FRAME);
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
                    w.show();
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
                if w <= 10 || h <= 10 {
                    return true;
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
                new_frame.set_image_scaled(Some(cropped_rgb_image.clone()));
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
                                // win.hide();
                                Window::delete(new_win_c);
                                true
                            } else {
                                false
                            }
                        }
                        Event::Push => {
                            offset = (event_x() - win.x(), event_y() - win.y());
                            true
                        }
                        Event::Drag => {
                            let mut drag_win_c = drag_wind_clone.lock().unwrap();
                            // let mut drag_frame = drag_frame_clone.lock().unwrap();
                            if let Some(drag_win_cc) = drag_win_c.as_mut() {
                                drag_win_cc.set_size(
                                    cropped_rgb_image.width(),
                                    cropped_rgb_image.height(),
                                );
                                drag_win_cc.set_pos(event_x() - offset.0, event_y() - offset.1);
                                drag_win_cc.add(&new_frame.clone());
                                drag_win_cc.show();
                                if win.width() > 0 && win.height() > 0 {
                                    win.set_size(0, 0);
                                }
                            }
                            true
                        }

                        Event::Released => {
                            let mut drag_win = drag_wind_clone.lock().unwrap();
                            if let Some(drag_win) = drag_win.as_mut() {
                                win.set_size(cropped_rgb_image.width(), cropped_rgb_image.height());
                                win.set_pos(drag_win.x(), drag_win.y());
                                win.add(&new_frame);
                                win.show();
                                drag_win.hide();
                            }
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
                win.set_pos(start_x, start_y);
                win.set_size(end_x - start_x, end_y - start_y);
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
                        win.make_current();
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
