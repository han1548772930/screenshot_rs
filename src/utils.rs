use std::ptr::null_mut;
use winapi::shared::windef::{DPI_AWARENESS_CONTEXT_SYSTEM_AWARE, HDC, HWND};
use winapi::um::winuser::{GetDC, ReleaseDC, SetThreadDpiAwarenessContext};
use winapi::um::wingdi::{GetDeviceCaps, HORZRES, LOGPIXELSX, LOGPIXELSY, VERTRES};

pub fn get_win_info() -> (i32, i32, i32, i32, f32) {
    let (mut w, mut h, mut w2, mut h2) = (0, 0, 0, 0);
    let mut proportion = 0.0;
    unsafe {
        unsafe {
            SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_SYSTEM_AWARE);
            let hdc: HDC = GetDC(null_mut());
            let width: i32 = GetDeviceCaps(hdc, HORZRES);
            let height: i32 = GetDeviceCaps(hdc, VERTRES);
            println!("{},{}", width, height);
            let dpi_x: i32 = GetDeviceCaps(hdc, LOGPIXELSX);
            let dpi_y: i32 = GetDeviceCaps(hdc, LOGPIXELSY);
            println!("{},{}", dpi_x, dpi_y);
            // 获取设备上下文
            let hdc = GetDC(null_mut());
            // 获取原始分辨率
            let width = GetDeviceCaps(hdc, HORZRES);
            let height = GetDeviceCaps(hdc, VERTRES);

            // 获取DPI
            let dpi_x = GetDeviceCaps(hdc, LOGPIXELSX);
            let dpi_y = GetDeviceCaps(hdc, LOGPIXELSY);

            // 计算缩放后的分辨率
            let scale_x = dpi_x as f32 / dpi_x as f32;
            let scale_y = dpi_y as f32 / dpi_y as f32;
            let scaled_width = width as f32 * scale_x;
            let scaled_height = height as f32 * scale_y;
            w = width as i32;
            h = height as i32;

            w2 = scaled_width as i32;
            h2 = scaled_height as i32;
            // 释放设备上下文
            ReleaseDC(null_mut(), hdc);

            proportion = format!("{:.8}", width as f64 / scaled_width as f64).parse::<f32>().unwrap();
        }
    }
    (w, h, w2, h2, proportion)
}