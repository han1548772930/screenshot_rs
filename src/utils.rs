use std::ptr::null_mut;
use winapi::shared::windef::{DPI_AWARENESS_CONTEXT_SYSTEM_AWARE, HDC, HWND};
use winapi::um::wingdi::{GetDeviceCaps, HORZRES, LOGPIXELSX, LOGPIXELSY, VERTRES};
use winapi::um::winuser::{GetDC, ReleaseDC, SetThreadDpiAwarenessContext};

pub fn get_win_info() -> (i32, i32, f32) {
    let (mut w, mut h) = (0, 0);
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

            // 计算缩放后的分辨率
            let scale_x = dpi_x as f32 / 96.0;
            let scale_y = dpi_y as f32 / 96.0;
            let scaled_width = width as f32 * scale_x;
            w = width as i32;
            h = height as i32;

            // 释放设备上下文
            ReleaseDC(null_mut(), hdc);
            proportion = format!("{:.8}", width as f64 / scaled_width as f64)
                .parse::<f32>()
                .unwrap();
        }
    }
    (w, h, proportion)
}
