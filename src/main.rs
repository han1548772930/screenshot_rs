#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use display_info::DisplayInfo;
use freya::prelude::*;
use screenshots::Screen;
use skia_safe::{
    AlphaType, Color, ColorType, Data, Image as SkiaImage, ImageInfo, Paint, PaintStyle,
    PathEffect, Rect, canvas::SrcRectConstraint,
};
use std::sync::Arc;
use winit::window::WindowLevel;

fn main() {
    let display_infos = DisplayInfo::all().unwrap();
    let dpi_scale = display_infos.first().unwrap().scale_factor;

    launch_cfg(
        app,
        LaunchConfig::<f32>::new()
            .with_decorations(false)
            .with_state(dpi_scale)
            .with_transparency(false)
            .with_window_attributes(|x| {
                x.with_window_level(WindowLevel::AlwaysOnTop)
                    .with_resizable(false)
            }),
    );
}

// 定义调整大小的手柄类型
#[derive(Debug, Clone, Copy, PartialEq)]
enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
}

fn app() -> Element {
    let platform = use_platform();
    let dpi_scale = consume_context::<f32>();

    let mut mouse_pos = use_signal(|| (0.0f32, 0.0f32));
    let (reference, size) = use_node_signal();
    let mut screenshot_data = use_signal::<Option<Arc<Vec<u8>>>>(|| None);
    let mut screen_size = use_signal(|| (0u32, 0u32));
    let mut screenshot_image = use_signal::<Option<SkiaImage>>(|| None);

    // 框选相关状态
    let mut is_selecting = use_signal(|| false);
    let mut selection_start = use_signal::<Option<(f32, f32)>>(|| None);
    let mut selection_end = use_signal::<Option<(f32, f32)>>(|| None);
    let mut current_selection = use_signal::<Option<((f32, f32), (f32, f32))>>(|| None);

    // 拖动相关状态
    let mut is_dragging = use_signal(|| false);
    let mut drag_offset = use_signal::<Option<(f32, f32)>>(|| None);

    // 调整大小相关状态
    let mut is_resizing = use_signal(|| false);
    let mut resize_handle = use_signal::<Option<ResizeHandle>>(|| None);
    let mut resize_anchor = use_signal::<Option<(f32, f32)>>(|| None); // 调整大小时的固定点

    use_effect(move || {
        platform.with_window(|w| {
            w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            w.set_cursor_visible(true);
            w.focus_window();
        });
    });

    let take_screenshot = move || {
        spawn(async move {
            if let Ok(screens) = Screen::all() {
                if let Some(screen) = screens.first() {
                    if let Ok(image) = screen.capture() {
                        let width = image.width();
                        let height = image.height();
                        let data = image.into_raw();

                        screen_size.set((width, height));
                        screenshot_data.set(Some(Arc::new(data.clone())));

                        let image_info = ImageInfo::new(
                            (width as i32, height as i32),
                            ColorType::RGBA8888,
                            AlphaType::Unpremul,
                            None,
                        );

                        let skia_data = Data::new_copy(&data);
                        if let Some(skia_img) = SkiaImage::from_raster_data(
                            &image_info,
                            skia_data,
                            (width * 4) as usize,
                        ) {
                            screenshot_image.set(Some(skia_img));
                        }
                        println!("截屏成功: {}x{}", width, height);
                    }
                }
            }
        });
    };

    use_effect(move || {
        take_screenshot();
    });

    // 检查点是否在矩形内的函数
    let point_in_rect = |x: f32, y: f32, rect_start: (f32, f32), rect_end: (f32, f32)| -> bool {
        let left = rect_start.0.min(rect_end.0);
        let right = rect_start.0.max(rect_end.0);
        let top = rect_start.1.min(rect_end.1);
        let bottom = rect_start.1.max(rect_end.1);

        x >= left && x <= right && y >= top && y <= bottom
    };

    // 检查点是否在调整大小手柄上
    let get_resize_handle =
        |x: f32, y: f32, rect_start: (f32, f32), rect_end: (f32, f32)| -> Option<ResizeHandle> {
            let left = rect_start.0.min(rect_end.0);
            let right = rect_start.0.max(rect_end.0);
            let top = rect_start.1.min(rect_end.1);
            let bottom = rect_start.1.max(rect_end.1);

            let handle_size = 8.0; // 手柄检测区域大小

            // 四个角的手柄
            if (x - left).abs() <= handle_size && (y - top).abs() <= handle_size {
                return Some(ResizeHandle::TopLeft);
            }
            if (x - right).abs() <= handle_size && (y - top).abs() <= handle_size {
                return Some(ResizeHandle::TopRight);
            }
            if (x - right).abs() <= handle_size && (y - bottom).abs() <= handle_size {
                return Some(ResizeHandle::BottomRight);
            }
            if (x - left).abs() <= handle_size && (y - bottom).abs() <= handle_size {
                return Some(ResizeHandle::BottomLeft);
            }

            // 四条边中点的手柄
            let center_x = (left + right) / 2.0;
            let center_y = (top + bottom) / 2.0;

            if (x - center_x).abs() <= handle_size && (y - top).abs() <= handle_size {
                return Some(ResizeHandle::Top);
            }
            if (x - right).abs() <= handle_size && (y - center_y).abs() <= handle_size {
                return Some(ResizeHandle::Right);
            }
            if (x - center_x).abs() <= handle_size && (y - bottom).abs() <= handle_size {
                return Some(ResizeHandle::Bottom);
            }
            if (x - left).abs() <= handle_size && (y - center_y).abs() <= handle_size {
                return Some(ResizeHandle::Left);
            }

            None
        };
    let point_in_buttons = |x: f32, y: f32, selection: ((f32, f32), (f32, f32))| -> bool {
        let ((start_x, start_y), (end_x, end_y)) = selection;
        let left = start_x.min(end_x);
        let right = start_x.max(end_x);
        let bottom = start_y.max(end_y);
        let center_x = (left + right) / 2.0;

        // 按钮参数（与绘制时保持一致）
        let toolbar_y = bottom + 15.0;
        let button_width = 40.0;
        let button_height = 30.0;
        let button_spacing = 5.0;
        let total_buttons = 5.0;
        let total_width = total_buttons * button_width + (total_buttons - 1.0) * button_spacing;
        let toolbar_start_x = center_x - total_width / 2.0;

        // 检查是否在按钮区域内
        x >= toolbar_start_x
            && x <= toolbar_start_x + total_width
            && y >= toolbar_y
            && y <= toolbar_y + button_height
    };

    let get_cursor_icon = move || -> CursorIcon {
        let (x, y) = *mouse_pos.read();

        // 根据当前状态确定光标
        if *is_selecting.read() {
            CursorIcon::Crosshair
        } else if *is_dragging.read() {
            CursorIcon::Move
        } else if *is_resizing.read() {
            // 根据调整手柄类型确定光标
            match *resize_handle.read() {
                Some(ResizeHandle::TopLeft) | Some(ResizeHandle::BottomRight) => {
                    CursorIcon::NwResize
                }
                Some(ResizeHandle::TopRight) | Some(ResizeHandle::BottomLeft) => {
                    CursorIcon::NeResize
                }
                Some(ResizeHandle::Top) | Some(ResizeHandle::Bottom) => CursorIcon::NsResize,
                Some(ResizeHandle::Left) | Some(ResizeHandle::Right) => CursorIcon::EwResize,
                None => CursorIcon::Default,
            }
        } else {
            // 悬停检测（空闲状态）
            if let Some(((start_x, start_y), (end_x, end_y))) = *current_selection.read() {
                if let Some(handle) = get_resize_handle(x, y, (start_x, start_y), (end_x, end_y)) {
                    // 悬停在调整手柄上
                    match handle {
                        ResizeHandle::TopLeft | ResizeHandle::BottomRight => CursorIcon::NwResize,
                        ResizeHandle::TopRight | ResizeHandle::BottomLeft => CursorIcon::NeResize,
                        ResizeHandle::Top | ResizeHandle::Bottom => CursorIcon::NsResize,
                        ResizeHandle::Left | ResizeHandle::Right => CursorIcon::EwResize,
                    }
                } else if point_in_rect(x, y, (start_x, start_y), (end_x, end_y)) {
                    // 悬停在选择框内
                    CursorIcon::Move
                } else if point_in_buttons(x, y, ((start_x, start_y), (end_x, end_y))) {
                    // 悬停在按钮区域 - 允许点击
                    CursorIcon::Pointer
                } else {
                    // 悬停在选择框外且不在按钮区域 - 禁止点击
                    CursorIcon::NotAllowed
                }
            } else {
                // 没有选择框
                CursorIcon::Default
            }
        }
    };

    let canvas = use_canvas(move || {
        platform.invalidate_drawing_area(size.peek().area);
        platform.request_animation_frame();

        let screenshot = screenshot_image.read().clone();
        let is_sel = *is_selecting.read();
        let is_drag = *is_dragging.read();
        let is_resize = *is_resizing.read();
        let sel_start = *selection_start.read();
        let sel_end = *selection_end.read();
        let current_sel = *current_selection.read();

        move |ctx| {
            ctx.canvas.clear(Color::TRANSPARENT);

            // 绘制截屏背景
            if let Some(img) = &screenshot {
                let canvas_width = ctx.area.width();
                let canvas_height = ctx.area.height();
                let dest_rect = Rect::from_xywh(0.0, 0.0, canvas_width, canvas_height);

                // 绘制完整背景图像
                let background_paint = Paint::default();
                ctx.canvas
                    .draw_image_rect(img, None, dest_rect, &background_paint);

                // 叠加黑色遮罩
                let mut mask_paint = Paint::default();
                mask_paint.set_color(Color::from_argb(160, 0, 0, 0));
                ctx.canvas.draw_rect(dest_rect, &mask_paint);

                // 处理选择区域
                let active_selection = if is_sel {
                    if let (Some((start_x, start_y)), Some((end_x, end_y))) = (sel_start, sel_end) {
                        Some(((start_x, start_y), (end_x, end_y)))
                    } else {
                        None
                    }
                } else {
                    current_sel
                };

                if let Some(((start_x, start_y), (end_x, end_y))) = active_selection {
                    let left = start_x.min(end_x);
                    let right = start_x.max(end_x);
                    let top = start_y.min(end_y);
                    let bottom = start_y.max(end_y);

                    // 确保选择区域在画布范围内
                    let clipped_left = left.max(0.0);
                    let clipped_top = top.max(0.0);
                    let clipped_right = right.min(canvas_width);
                    let clipped_bottom = bottom.min(canvas_height);

                    // 只有当选择区域有效时才绘制
                    if clipped_right > clipped_left && clipped_bottom > clipped_top {
                        let selection_rect = Rect::from_xywh(
                            clipped_left,
                            clipped_top,
                            clipped_right - clipped_left,
                            clipped_bottom - clipped_top,
                        );

                        // 绘制清晰的选择区域（移除遮罩）
                        let clear_paint = Paint::default();
                        let src_rect = Rect::from_xywh(
                            clipped_left,
                            clipped_top,
                            clipped_right - clipped_left,
                            clipped_bottom - clipped_top,
                        );

                        ctx.canvas.draw_image_rect(
                            img,
                            Some((&src_rect, SrcRectConstraint::Fast)),
                            selection_rect,
                            &clear_paint,
                        );

                        // 绘制选择框边框
                        let mut selection_paint = Paint::default();
                        selection_paint.set_style(PaintStyle::Stroke);
                        selection_paint.set_anti_alias(true);

                        match (is_sel, is_drag, is_resize) {
                            (true, _, _) => {
                                // 正在选择 - 绿色虚线
                                selection_paint.set_color(Color::from_rgb(0, 255, 0));
                                selection_paint.set_stroke_width(1.0);
                                if let Some(dash_effect) = PathEffect::dash(&[8.0, 4.0], 0.0) {
                                    selection_paint.set_path_effect(dash_effect);
                                }
                            }
                            _ => {
                                selection_paint.set_color(Color::from_rgb(0, 255, 255));
                                selection_paint.set_stroke_width(1.0);
                            }
                        }

                        // 使用原始的选择框坐标绘制边框（不裁剪）
                        let border_rect = Rect::from_xywh(left, top, right - left, bottom - top);
                        ctx.canvas.draw_rect(border_rect, &selection_paint);

                        // 调试十字线（仅在选择时显示）
                        if is_sel {
                            let mut debug_paint = Paint::default();
                            debug_paint.set_color(Color::from_rgb(255, 255, 0));
                            debug_paint.set_stroke_width(2.0);
                            debug_paint.set_anti_alias(true);

                            let cross_size = 15.0;
                            ctx.canvas.draw_line(
                                (end_x - cross_size, end_y),
                                (end_x + cross_size, end_y),
                                &debug_paint,
                            );
                            ctx.canvas.draw_line(
                                (end_x, end_y - cross_size),
                                (end_x, end_y + cross_size),
                                &debug_paint,
                            );
                            ctx.canvas.draw_circle((end_x, end_y), 3.0, &debug_paint);
                        }

                        // 绘制调整大小的手柄（仅在选择完成且没有正在进行其他操作时显示）
                        // ...existing code...

                        // 绘制调整大小的手柄（仅在选择完成且没有正在进行其他操作时显示）
                        if !is_sel && !is_drag && !is_resize {
                            let mut handle_paint = Paint::default();
                            handle_paint.set_color(Color::from_rgb(255, 255, 255));
                            handle_paint.set_anti_alias(true);

                            let mut handle_border_paint = Paint::default();
                            handle_border_paint.set_color(Color::from_rgb(0, 0, 0));
                            handle_border_paint.set_style(PaintStyle::Stroke);
                            handle_border_paint.set_stroke_width(1.0);
                            handle_border_paint.set_anti_alias(true);

                            let handle_size = 6.0; // 增加手柄大小
                            let center_x = (left + right) / 2.0;
                            let center_y = (top + bottom) / 2.0;

                            // 绘制8个调整大小的手柄
                            let handles = [
                                (left, top),        // TopLeft
                                (center_x, top),    // Top
                                (right, top),       // TopRight
                                (right, center_y),  // Right
                                (right, bottom),    // BottomRight
                                (center_x, bottom), // Bottom
                                (left, bottom),     // BottomLeft
                                (left, center_y),   // Left
                            ];

                            for (handle_x, handle_y) in handles {
                                // 创建小方点的矩形
                                let handle_rect = Rect::from_xywh(
                                    handle_x - handle_size,
                                    handle_y - handle_size,
                                    handle_size * 2.0,
                                    handle_size * 2.0,
                                );

                                // 绘制白色填充方形
                                ctx.canvas.draw_rect(handle_rect, &handle_paint);
                                // 绘制黑色边框
                                ctx.canvas.draw_rect(handle_rect, &handle_border_paint);
                            }

                            // 绘制拖动提示图标
                            let center_x = (left + right) / 2.0;
                            let center_y = (top + bottom) / 2.0;

                            let mut hint_paint = Paint::default();
                            hint_paint.set_color(Color::from_argb(120, 255, 255, 255));
                            hint_paint.set_anti_alias(true);
                            hint_paint.set_stroke_width(2.0);

                            // 绘制十字移动图标
                            let arrow_size = 10.0;
                            ctx.canvas.draw_line(
                                (center_x - arrow_size, center_y),
                                (center_x + arrow_size, center_y),
                                &hint_paint,
                            );
                            ctx.canvas.draw_line(
                                (center_x, center_y - arrow_size),
                                (center_x, center_y + arrow_size),
                                &hint_paint,
                            );

                            // 绘制工具栏按钮
                            let toolbar_y = bottom + 15.0; // 按钮距离选择框底部15像素
                            let button_width = 40.0;
                            let button_height = 30.0;
                            let button_spacing = 5.0;

                            // 计算工具栏起始位置（居中对齐）
                            let total_buttons = 5.0;
                            let total_width = total_buttons * button_width
                                + (total_buttons - 1.0) * button_spacing;
                            let toolbar_start_x = center_x - total_width / 2.0;

                            // 按钮样式
                            let mut button_paint = Paint::default();
                            button_paint.set_color(Color::from_argb(220, 45, 45, 45)); // 半透明深灰色
                            button_paint.set_anti_alias(true);

                            let mut button_border_paint = Paint::default();
                            button_border_paint.set_color(Color::from_rgb(180, 180, 180));
                            button_border_paint.set_style(PaintStyle::Stroke);
                            button_border_paint.set_stroke_width(1.0);
                            button_border_paint.set_anti_alias(true);

                            let mut icon_paint = Paint::default();
                            icon_paint.set_color(Color::from_rgb(255, 255, 255));
                            icon_paint.set_stroke_width(2.0);
                            icon_paint.set_anti_alias(true);

                            // 按钮定义：[图标类型, x偏移]
                            let buttons = [
                                ("save", 0.0),  // 保存
                                ("copy", 1.0),  // 复制
                                ("edit", 2.0),  // 编辑
                                ("share", 3.0), // 分享
                                ("close", 4.0), // 关闭
                            ];

                            for (icon_type, index) in buttons {
                                let button_x =
                                    toolbar_start_x + index * (button_width + button_spacing);
                                let button_rect = Rect::from_xywh(
                                    button_x,
                                    toolbar_y,
                                    button_width,
                                    button_height,
                                );

                                // 绘制按钮背景
                                ctx.canvas.draw_round_rect(
                                    button_rect,
                                    4.0, // 圆角半径
                                    4.0,
                                    &button_paint,
                                );
                                ctx.canvas.draw_round_rect(
                                    button_rect,
                                    4.0,
                                    4.0,
                                    &button_border_paint,
                                );

                                // 绘制图标
                                let icon_center_x = button_x + button_width / 2.0;
                                let icon_center_y = toolbar_y + button_height / 2.0;
                                let icon_size = 8.0;

                                match icon_type {
                                    "save" => {
                                        // 绘制保存图标（磁盘）
                                        let disk_rect = Rect::from_xywh(
                                            icon_center_x - icon_size,
                                            icon_center_y - icon_size,
                                            icon_size * 2.0,
                                            icon_size * 2.0,
                                        );
                                        ctx.canvas.draw_rect(disk_rect, &icon_paint);
                                        // 绘制磁盘标签
                                        let label_rect = Rect::from_xywh(
                                            icon_center_x - icon_size * 0.6,
                                            icon_center_y - icon_size * 0.8,
                                            icon_size * 1.2,
                                            icon_size * 0.4,
                                        );
                                        let mut bg_paint = Paint::default();
                                        bg_paint.set_color(Color::from_rgb(45, 45, 45));
                                        ctx.canvas.draw_rect(label_rect, &bg_paint);
                                    }
                                    "copy" => {
                                        // 绘制复制图标（两个重叠的方框）
                                        let rect1 = Rect::from_xywh(
                                            icon_center_x - icon_size,
                                            icon_center_y - icon_size,
                                            icon_size * 1.5,
                                            icon_size * 1.5,
                                        );
                                        let rect2 = Rect::from_xywh(
                                            icon_center_x - icon_size * 0.5,
                                            icon_center_y - icon_size * 0.5,
                                            icon_size * 1.5,
                                            icon_size * 1.5,
                                        );
                                        icon_paint.set_style(PaintStyle::Stroke);
                                        ctx.canvas.draw_rect(rect1, &icon_paint);
                                        ctx.canvas.draw_rect(rect2, &icon_paint);
                                        icon_paint.set_style(PaintStyle::Fill);
                                    }
                                    "edit" => {
                                        // 绘制编辑图标（铅笔）
                                        ctx.canvas.draw_line(
                                            (icon_center_x - icon_size, icon_center_y + icon_size),
                                            (icon_center_x + icon_size, icon_center_y - icon_size),
                                            &icon_paint,
                                        );
                                        // 铅笔头
                                        ctx.canvas.draw_circle(
                                            (
                                                icon_center_x + icon_size * 0.7,
                                                icon_center_y - icon_size * 0.7,
                                            ),
                                            2.0,
                                            &icon_paint,
                                        );
                                    }
                                    "share" => {
                                        // 绘制分享图标（箭头）
                                        ctx.canvas.draw_line(
                                            (icon_center_x - icon_size, icon_center_y),
                                            (icon_center_x + icon_size, icon_center_y),
                                            &icon_paint,
                                        );
                                        // 箭头头部
                                        ctx.canvas.draw_line(
                                            (icon_center_x + icon_size, icon_center_y),
                                            (
                                                icon_center_x + icon_size * 0.5,
                                                icon_center_y - icon_size * 0.5,
                                            ),
                                            &icon_paint,
                                        );
                                        ctx.canvas.draw_line(
                                            (icon_center_x + icon_size, icon_center_y),
                                            (
                                                icon_center_x + icon_size * 0.5,
                                                icon_center_y + icon_size * 0.5,
                                            ),
                                            &icon_paint,
                                        );
                                    }
                                    "close" => {
                                        // 绘制关闭图标（X）
                                        ctx.canvas.draw_line(
                                            (
                                                icon_center_x - icon_size * 0.7,
                                                icon_center_y - icon_size * 0.7,
                                            ),
                                            (
                                                icon_center_x + icon_size * 0.7,
                                                icon_center_y + icon_size * 0.7,
                                            ),
                                            &icon_paint,
                                        );
                                        ctx.canvas.draw_line(
                                            (
                                                icon_center_x + icon_size * 0.7,
                                                icon_center_y - icon_size * 0.7,
                                            ),
                                            (
                                                icon_center_x - icon_size * 0.7,
                                                icon_center_y + icon_size * 0.7,
                                            ),
                                            &icon_paint,
                                        );
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // 键盘事件
    let onglobalkeydown = move |e: KeyboardEvent| {
        println!("按键事件: {:?}", e.key);
        match e.key {
            Key::Escape => {
                println!("Esc键被按下，正在退出...");
                platform.exit();
            }
            _ => {}
        }
    };

    // 鼠标事件处理
    let onmousedown = move |e: MouseEvent| {
        // 右键退出
        if e.trigger_button == Some(MouseButton::Right) {
            println!("右键点击，正在退出...");
            platform.exit();
            return;
        }

        let element_coords = e.get_element_coordinates();
        let x = element_coords.x as f32 * dpi_scale;
        let y = element_coords.y as f32 * dpi_scale;

        println!("=== 鼠标按下 ===");
        println!("坐标: ({}, {})", x, y);

        // 先读取选择框状态，避免借用冲突
        let current_sel = *current_selection.read();

        // 检查是否点击在现有选择框上
        if let Some(((start_x, start_y), (end_x, end_y))) = current_sel {
            // 检查是否点击在按钮上
            if point_in_buttons(x, y, ((start_x, start_y), (end_x, end_y))) {
                // 计算点击的是哪个按钮
                let left = start_x.min(end_x);
                let right = start_x.max(end_x);
                let bottom = start_y.max(end_y);
                let center_x = (left + right) / 2.0;

                let toolbar_y = bottom + 15.0;
                let button_width = 40.0;
                let button_spacing = 5.0;
                let total_buttons = 5.0;
                let total_width =
                    total_buttons * button_width + (total_buttons - 1.0) * button_spacing;
                let toolbar_start_x = center_x - total_width / 2.0;

                // 计算按钮索引
                let relative_x = x - toolbar_start_x;
                let button_index = (relative_x / (button_width + button_spacing)).floor() as usize;

                match button_index {
                    0 => {
                        println!("点击了保存按钮");
                        // 保存截图功能
                        if let Some(screenshot) = screenshot_image.read().clone() {
                            println!(
                                "保存区域: ({}, {}) 到 ({}, {})",
                                start_x, start_y, end_x, end_y
                            );
                            // TODO: 实现保存选定区域到文件
                        }
                    }
                    1 => {
                        println!("点击了复制按钮");
                        // 复制到剪贴板功能
                        if let Some(screenshot) = screenshot_image.read().clone() {
                            println!(
                                "复制区域: ({}, {}) 到 ({}, {})",
                                start_x, start_y, end_x, end_y
                            );
                            // TODO: 实现复制选定区域到剪贴板
                        }
                    }
                    2 => {
                        println!("点击了编辑按钮");
                        // 进入编辑模式
                        println!(
                            "进入编辑模式，选择区域: ({}, {}) 到 ({}, {})",
                            start_x, start_y, end_x, end_y
                        );
                        // TODO: 实现编辑功能（添加文字、箭头、马赛克等）
                    }
                    3 => {
                        println!("点击了分享按钮");
                        // 分享功能
                        if let Some(screenshot) = screenshot_image.read().clone() {
                            println!(
                                "分享区域: ({}, {}) 到 ({}, {})",
                                start_x, start_y, end_x, end_y
                            );
                            // TODO: 实现分享功能（保存到临时文件并打开分享对话框）
                        }
                    }
                    4 => {
                        println!("点击了关闭按钮");
                        // 清除选择框并返回到初始状态
                        // current_selection.set(None);
                        is_selecting.set(false);
                        is_dragging.set(false);
                        is_resizing.set(false);
                        drag_offset.set(None);
                        resize_handle.set(None);
                        resize_anchor.set(None);
                        println!("已清除选择框");
                    }
                    _ => {
                        println!("点击了未知按钮: {}", button_index);
                    }
                }
                return;
            }

            // 首先检查是否点击在调整大小手柄上
            if let Some(handle) = get_resize_handle(x, y, (start_x, start_y), (end_x, end_y)) {
                println!("开始调整大小: {:?}", handle);
                is_resizing.set(true);
                resize_handle.set(Some(handle));

                // 设置锚点（调整大小时的固定点）
                let left = start_x.min(end_x);
                let right = start_x.max(end_x);
                let top = start_y.min(end_y);
                let bottom = start_y.max(end_y);

                let anchor = match handle {
                    ResizeHandle::TopLeft => (right, bottom),
                    ResizeHandle::TopRight => (left, bottom),
                    ResizeHandle::BottomRight => (left, top),
                    ResizeHandle::BottomLeft => (right, top),
                    ResizeHandle::Top => (left, bottom),
                    ResizeHandle::Bottom => (left, top),
                    ResizeHandle::Left => (right, top),
                    ResizeHandle::Right => (left, top),
                };
                resize_anchor.set(Some(anchor));
                return;
            }
            // 然后检查是否点击在选择框内（开始拖动）
            else if point_in_rect(x, y, (start_x, start_y), (end_x, end_y)) {
                is_dragging.set(true);
                let offset_x = x - start_x.min(end_x);
                let offset_y = y - start_y.min(end_y);
                drag_offset.set(Some((offset_x, offset_y)));
                println!("开始拖动选择框，偏移: ({}, {})", offset_x, offset_y);
                return;
            }
            // 如果点击在选择框外且不在按钮区域，忽略点击事件
            else {
                println!("点击在选择框外，忽略点击事件");
                return;
            }
        }

        // 只有在没有现有选择框的情况下才开始新的选择
        selection_start.set(Some((x, y)));
        selection_end.set(Some((x, y)));
        is_selecting.set(true);
        current_selection.set(None);
        is_dragging.set(false);
        is_resizing.set(false);
        drag_offset.set(None);
        resize_handle.set(None);
        resize_anchor.set(None);

        println!("开始新选择: ({}, {})", x, y);
    };

    let onmousemove = move |e: MouseEvent| {
        let element_coords = e.get_element_coordinates();
        let x = element_coords.x as f32 * dpi_scale;
        let y = element_coords.y as f32 * dpi_scale;
        mouse_pos.set((x, y));
        if *is_selecting.read() {
            // 正在框选
            selection_end.set(Some((x, y)));
        } else if *is_dragging.read() {
            // 正在拖动
            let current_sel = *current_selection.read();
            let drag_off = *drag_offset.read();

            if let (Some(((start_x, start_y), (end_x, end_y))), Some((offset_x, offset_y))) =
                (current_sel, drag_off)
            {
                let width = (end_x - start_x).abs();
                let height = (end_y - start_y).abs();

                let new_left = x - offset_x;
                let new_top = y - offset_y;
                let new_right = new_left + width;
                let new_bottom = new_top + height;

                current_selection.set(Some(((new_left, new_top), (new_right, new_bottom))));
            }
        } else if *is_resizing.read() {
            // 正在调整大小
            let handle = *resize_handle.read();
            let anchor = *resize_anchor.read();

            // 先读取current_selection的值，然后释放借用
            let current_sel = *current_selection.read();

            if let (
                Some(handle),
                Some((anchor_x, anchor_y)),
                Some(((start_x, start_y), (end_x, end_y))),
            ) = (handle, anchor, current_sel)
            {
                let current_left = start_x.min(end_x);
                let current_right = start_x.max(end_x);
                let current_top = start_y.min(end_y);
                let current_bottom = start_y.max(end_y);

                let (new_start, new_end) = match handle {
                    // 角手柄：一个角拖动，对角为锚点
                    ResizeHandle::TopLeft => ((x, y), (anchor_x, anchor_y)),
                    ResizeHandle::TopRight => ((anchor_x, y), (x, anchor_y)),
                    ResizeHandle::BottomRight => ((anchor_x, anchor_y), (x, y)),
                    ResizeHandle::BottomLeft => ((x, anchor_y), (anchor_x, y)),

                    // 边手柄：只改变一个维度，保持另一个维度不变
                    ResizeHandle::Top => ((current_left, y), (current_right, current_bottom)),
                    ResizeHandle::Bottom => ((current_left, current_top), (current_right, y)),
                    ResizeHandle::Left => ((x, current_top), (current_right, current_bottom)),
                    ResizeHandle::Right => ((current_left, current_top), (x, current_bottom)),
                };

                // 现在可以安全地进行可变借用
                current_selection.set(Some((new_start, new_end)));
            }
        }
    };
    let onmouseup = move |e: MouseEvent| {
        let element_coords = e.get_element_coordinates();
        let x = element_coords.x as f32 * dpi_scale;
        let y = element_coords.y as f32 * dpi_scale;

        if *is_selecting.read() {
            // 完成框选
            if let Some(start) = *selection_start.read() {
                current_selection.set(Some((start, (x, y))));
                println!("选择完成: 从({}, {}) 到 ({}, {})", start.0, start.1, x, y);
            }
            is_selecting.set(false);
        } else if *is_dragging.read() {
            // 完成拖动
            if let Some(((start_x, start_y), (end_x, end_y))) = *current_selection.read() {
                println!(
                    "拖动完成: 新位置从({}, {}) 到 ({}, {})",
                    start_x, start_y, end_x, end_y
                );
            }
            is_dragging.set(false);
            drag_offset.set(None);
        } else if *is_resizing.read() {
            // 完成调整大小
            if let Some(((start_x, start_y), (end_x, end_y))) = *current_selection.read() {
                println!(
                    "调整大小完成: 新尺寸从({}, {}) 到 ({}, {})",
                    start_x, start_y, end_x, end_y
                );
            }
            is_resizing.set(false);
            resize_handle.set(None);
            resize_anchor.set(None);
        }
    };

    rsx!(
        rect {
            width: "fill",
            height: "fill",
            onmousedown,
            onmousemove,
            onmouseup,
            onglobalkeydown,
                CursorArea {
                    icon: get_cursor_icon(),
                    rect {
                    canvas_reference: canvas.attribute(),
                    reference,
                     width: "fill",
                     height: "fill",
                    }
                }
        }
    )
}
