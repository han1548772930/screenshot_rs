#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use display_info::DisplayInfo;
use freya::prelude::*;
use screenshots::Screen;
use skia_safe::{
    AlphaType, Color, ColorType, Data, Image as SkiaImage, ImageInfo, Paint, PaintStyle,
    PathEffect, Rect, canvas::SrcRectConstraint, images,
};
use std::sync::Arc;
use winit::window::WindowLevel;

// 常量定义
mod constants {
    pub const HANDLE_SIZE: f32 = 6.0;
    pub const HANDLE_DETECT_SIZE: f32 = 8.0;
    pub const BUTTON_WIDTH: f32 = 40.0;
    pub const BUTTON_HEIGHT: f32 = 30.0;
    pub const BUTTON_SPACING: f32 = 5.0;
    pub const TOTAL_BUTTONS: f32 = 5.0;
    pub const MIN_SELECTION_SIZE: f32 = 10.0;
    pub const TOOLBAR_MARGIN: f32 = 15.0;
    pub const SCREEN_MARGIN: f32 = 10.0;
}

use constants::*;

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

#[derive(Debug, Clone, Copy, PartialEq)]
enum AppState {
    Selecting,
    Dragging,
    Resizing,
    Idle,
}

// 选择框结构
#[derive(Debug, Clone, Copy)]
struct Selection {
    start: (f32, f32),
    end: (f32, f32),
}

impl Selection {
    fn bounds(&self) -> (f32, f32, f32, f32) {
        let left = self.start.0.min(self.end.0);
        let right = self.start.0.max(self.end.0);
        let top = self.start.1.min(self.end.1);
        let bottom = self.start.1.max(self.end.1);
        (left, top, right, bottom)
    }

    fn center(&self) -> (f32, f32) {
        let (left, top, right, bottom) = self.bounds();
        ((left + right) / 2.0, (top + bottom) / 2.0)
    }

    fn size(&self) -> (f32, f32) {
        let (left, top, right, bottom) = self.bounds();
        (right - left, bottom - top)
    }
}

// 工具栏计算
struct Toolbar {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

impl Toolbar {
    fn calculate(selection: &Selection, screen_size: (u32, u32)) -> Self {
        let (left, top, right, bottom) = selection.bounds();
        let center_x = (left + right) / 2.0;

        let width = TOTAL_BUTTONS * BUTTON_WIDTH + (TOTAL_BUTTONS - 1.0) * BUTTON_SPACING;
        let height = BUTTON_HEIGHT;

        // 默认位置（选择框下方）
        let default_y = bottom + TOOLBAR_MARGIN;
        let toolbar_bottom = default_y + height;

        // 检查是否需要移动到上方
        let y = if toolbar_bottom > screen_size.1 as f32 - SCREEN_MARGIN {
            top - height - TOOLBAR_MARGIN
        } else {
            default_y
        }
        .max(SCREEN_MARGIN);

        // 水平居中，但不超出屏幕边界
        let x = (center_x - width / 2.0)
            .max(SCREEN_MARGIN)
            .min(screen_size.0 as f32 - width - SCREEN_MARGIN);

        Self {
            x,
            y,
            width,
            height,
        }
    }

    fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

    fn get_button_index(&self, x: f32, y: f32) -> Option<usize> {
        if !self.contains_point(x, y) {
            return None;
        }
        let relative_x = x - self.x;
        let index = (relative_x / (BUTTON_WIDTH + BUTTON_SPACING)).floor() as usize;
        if index < 5 { Some(index) } else { None }
    }
}

// 几何工具函数
mod geometry {
    use super::*;

    pub fn point_in_rect(x: f32, y: f32, selection: &Selection) -> bool {
        let (left, top, right, bottom) = selection.bounds();
        x >= left && x <= right && y >= top && y <= bottom
    }

    pub fn get_resize_handle(x: f32, y: f32, selection: &Selection) -> Option<ResizeHandle> {
        let (left, top, right, bottom) = selection.bounds();
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        // 检查角手柄
        if (x - left).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::TopLeft);
        }
        if (x - right).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::TopRight);
        }
        if (x - right).abs() <= HANDLE_DETECT_SIZE && (y - bottom).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::BottomRight);
        }
        if (x - left).abs() <= HANDLE_DETECT_SIZE && (y - bottom).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::BottomLeft);
        }

        // 检查边手柄
        if (x - center_x).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::Top);
        }
        if (x - right).abs() <= HANDLE_DETECT_SIZE && (y - center_y).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::Right);
        }
        if (x - center_x).abs() <= HANDLE_DETECT_SIZE && (y - bottom).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::Bottom);
        }
        if (x - left).abs() <= HANDLE_DETECT_SIZE && (y - center_y).abs() <= HANDLE_DETECT_SIZE {
            return Some(ResizeHandle::Left);
        }

        None
    }

    pub fn get_resize_anchor(handle: ResizeHandle, selection: &Selection) -> (f32, f32) {
        let (left, top, right, bottom) = selection.bounds();
        match handle {
            ResizeHandle::TopLeft => (right, bottom),
            ResizeHandle::TopRight => (left, bottom),
            ResizeHandle::BottomRight => (left, top),
            ResizeHandle::BottomLeft => (right, top),
            ResizeHandle::Top | ResizeHandle::Bottom => (
                left,
                if handle == ResizeHandle::Top {
                    bottom
                } else {
                    top
                },
            ),
            ResizeHandle::Left | ResizeHandle::Right => (
                if handle == ResizeHandle::Left {
                    right
                } else {
                    left
                },
                top,
            ),
        }
    }

    pub fn constrain_to_screen(selection: Selection, screen_size: (u32, u32)) -> Selection {
        let (width, height) = selection.size();
        let screen_w = screen_size.0 as f32;
        let screen_h = screen_size.1 as f32;

        let left = selection
            .start
            .0
            .min(selection.end.0)
            .max(0.0)
            .min(screen_w - width);
        let top = selection
            .start
            .1
            .min(selection.end.1)
            .max(0.0)
            .min(screen_h - height);

        Selection {
            start: (left, top),
            end: (left + width, top + height),
        }
    }
}

// 绘制工具
mod rendering {
    use freya::core::custom_attributes::CanvasRunnerContext;

    use super::*;

    pub fn draw_selection_area(
        ctx: &mut CanvasRunnerContext,
        img: &SkiaImage,
        selection: &Selection,
    ) {
        let (left, top, right, bottom) = selection.bounds();
        let canvas_width = ctx.area.width();
        let canvas_height = ctx.area.height();

        let clipped_left = left.max(0.0);
        let clipped_top = top.max(0.0);
        let clipped_right = right.min(canvas_width);
        let clipped_bottom = bottom.min(canvas_height);

        if clipped_right > clipped_left && clipped_bottom > clipped_top {
            let selection_rect = Rect::from_xywh(
                clipped_left,
                clipped_top,
                clipped_right - clipped_left,
                clipped_bottom - clipped_top,
            );
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
                &Paint::default(),
            );
        }
    }

    pub fn draw_selection_border(
        ctx: &mut CanvasRunnerContext,
        selection: &Selection,
        state: AppState,
    ) {
        let (left, top, right, bottom) = selection.bounds();
        let mut paint = Paint::default();
        paint.set_style(PaintStyle::Stroke);
        paint.set_anti_alias(true);
        paint.set_stroke_width(1.0);

        // 根据状态设置颜色
        let color = match state {
            AppState::Selecting => Color::from_rgb(0, 255, 0),
            _ => Color::from_rgb(0, 255, 255),
        };
        paint.set_color(color);

        // 如果是选择状态，添加虚线效果
        if state == AppState::Selecting {
            if let Some(dash_effect) = PathEffect::dash(&[8.0, 4.0], 0.0) {
                paint.set_path_effect(dash_effect);
            }
        }

        let rect = Rect::from_xywh(left, top, right - left, bottom - top);
        ctx.canvas.draw_rect(rect, &paint);
    }

    pub fn draw_handles(ctx: &mut CanvasRunnerContext, selection: &Selection) {
        let (left, top, right, bottom) = selection.bounds();
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        let mut handle_paint = Paint::default();
        handle_paint.set_color(Color::from_rgb(255, 255, 255));
        handle_paint.set_anti_alias(true);

        let mut border_paint = Paint::default();
        border_paint.set_color(Color::from_rgb(0, 0, 0));
        border_paint.set_style(PaintStyle::Stroke);
        border_paint.set_stroke_width(1.0);
        border_paint.set_anti_alias(true);

        let handles = [
            (left, top),
            (center_x, top),
            (right, top),
            (right, center_y),
            (right, bottom),
            (center_x, bottom),
            (left, bottom),
            (left, center_y),
        ];

        for (x, y) in handles {
            let rect = Rect::from_xywh(
                x - HANDLE_SIZE,
                y - HANDLE_SIZE,
                HANDLE_SIZE * 2.0,
                HANDLE_SIZE * 2.0,
            );
            ctx.canvas.draw_rect(rect, &handle_paint);
            ctx.canvas.draw_rect(rect, &border_paint);
        }
    }

    pub fn draw_toolbar(ctx: &mut CanvasRunnerContext, toolbar: &Toolbar, selection: &Selection) {
        let buttons = ["save", "copy", "edit", "share", "close"];

        let mut button_paint = Paint::default();
        button_paint.set_color(Color::from_argb(220, 45, 45, 45));
        button_paint.set_anti_alias(true);

        let mut border_paint = Paint::default();
        border_paint.set_color(Color::from_rgb(180, 180, 180));
        border_paint.set_style(PaintStyle::Stroke);
        border_paint.set_stroke_width(1.0);
        border_paint.set_anti_alias(true);

        for (i, icon_type) in buttons.iter().enumerate() {
            let button_x = toolbar.x + i as f32 * (BUTTON_WIDTH + BUTTON_SPACING);
            let button_rect = Rect::from_xywh(button_x, toolbar.y, BUTTON_WIDTH, BUTTON_HEIGHT);

            ctx.canvas
                .draw_round_rect(button_rect, 4.0, 4.0, &button_paint);
            ctx.canvas
                .draw_round_rect(button_rect, 4.0, 4.0, &border_paint);

            draw_icon(
                ctx,
                icon_type,
                button_x + BUTTON_WIDTH / 2.0,
                toolbar.y + BUTTON_HEIGHT / 2.0,
            );
        }
    }

    fn draw_icon(ctx: &mut CanvasRunnerContext, icon_type: &str, center_x: f32, center_y: f32) {
        let mut paint = Paint::default();
        paint.set_color(Color::from_rgb(255, 255, 255));
        paint.set_stroke_width(2.0);
        paint.set_anti_alias(true);

        let size = 8.0;

        match icon_type {
            "save" => {
                let rect =
                    Rect::from_xywh(center_x - size, center_y - size, size * 2.0, size * 2.0);
                ctx.canvas.draw_rect(rect, &paint);
            }
            "copy" => {
                paint.set_style(PaintStyle::Stroke);
                let rect1 =
                    Rect::from_xywh(center_x - size, center_y - size, size * 1.5, size * 1.5);
                let rect2 = Rect::from_xywh(
                    center_x - size * 0.5,
                    center_y - size * 0.5,
                    size * 1.5,
                    size * 1.5,
                );
                ctx.canvas.draw_rect(rect1, &paint);
                ctx.canvas.draw_rect(rect2, &paint);
            }
            "edit" => {
                ctx.canvas.draw_line(
                    (center_x - size, center_y + size),
                    (center_x + size, center_y - size),
                    &paint,
                );
                ctx.canvas
                    .draw_circle((center_x + size * 0.7, center_y - size * 0.7), 2.0, &paint);
            }
            "share" => {
                ctx.canvas.draw_line(
                    (center_x - size, center_y),
                    (center_x + size, center_y),
                    &paint,
                );
                ctx.canvas.draw_line(
                    (center_x + size, center_y),
                    (center_x + size * 0.5, center_y - size * 0.5),
                    &paint,
                );
                ctx.canvas.draw_line(
                    (center_x + size, center_y),
                    (center_x + size * 0.5, center_y + size * 0.5),
                    &paint,
                );
            }
            "close" => {
                ctx.canvas.draw_line(
                    (center_x - size * 0.7, center_y - size * 0.7),
                    (center_x + size * 0.7, center_y + size * 0.7),
                    &paint,
                );
                ctx.canvas.draw_line(
                    (center_x + size * 0.7, center_y - size * 0.7),
                    (center_x - size * 0.7, center_y + size * 0.7),
                    &paint,
                );
            }
            _ => {}
        }
    }
}
// 添加光标类型转换函数
fn resize_handle_to_cursor(handle: ResizeHandle) -> CursorIcon {
    match handle {
        ResizeHandle::TopLeft | ResizeHandle::BottomRight => CursorIcon::NwResize,
        ResizeHandle::TopRight | ResizeHandle::BottomLeft => CursorIcon::NeResize,
        ResizeHandle::Top | ResizeHandle::Bottom => CursorIcon::NsResize,
        ResizeHandle::Left | ResizeHandle::Right => CursorIcon::EwResize,
    }
}

// 在 app 函数中添加光标状态管理
fn app() -> Element {
    let platform = use_platform();
    let dpi_scale = consume_context::<f32>();

    // 状态管理（保持原有的）
    let mut screenshot_image = use_signal::<Option<SkiaImage>>(|| None);
    let mut screen_size = use_signal(|| (0u32, 0u32));
    let mut mouse_pos = use_signal(|| (0.0f32, 0.0f32));
    let mut app_state = use_signal(|| AppState::Idle);
    let mut current_selection = use_signal::<Option<Selection>>(|| None);
    let mut drag_offset = use_signal::<Option<(f32, f32)>>(|| None);
    let mut resize_handle = use_signal::<Option<ResizeHandle>>(|| None);
    let mut resize_anchor = use_signal::<Option<(f32, f32)>>(|| None);
    let mut temp_selection = use_signal::<Option<Selection>>(|| None);

    let (reference, size) = use_node_signal();

    // 添加光标状态计算函数
    let get_cursor_icon = move || -> CursorIcon {
        let (x, y) = *mouse_pos.read();
        let current_state = *app_state.read();

        match current_state {
            AppState::Selecting => CursorIcon::Crosshair,
            AppState::Dragging => CursorIcon::Move,
            AppState::Resizing => {
                if let Some(handle) = *resize_handle.read() {
                    resize_handle_to_cursor(handle)
                } else {
                    CursorIcon::Default
                }
            }
            AppState::Idle => {
                // 悬停检测（空闲状态）
                if let Some(selection) = *current_selection.read() {
                    let toolbar = Toolbar::calculate(&selection, *screen_size.read());

                    // 检查是否悬停在工具栏按钮上
                    if toolbar.contains_point(x, y) {
                        return CursorIcon::Pointer;
                    }

                    // 检查是否悬停在调整大小手柄上
                    if let Some(handle) = geometry::get_resize_handle(x, y, &selection) {
                        return resize_handle_to_cursor(handle);
                    }

                    // 检查是否悬停在选择框内
                    if geometry::point_in_rect(x, y, &selection) {
                        return CursorIcon::Move;
                    }

                    // 悬停在选择框外 - 禁止点击
                    CursorIcon::NotAllowed
                } else {
                    // 没有选择框
                    CursorIcon::Default
                }
            }
        }
    };

    // 初始化逻辑（保持不变）
    use_effect(move || {
        platform.with_window(|w| {
            w.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
            w.set_cursor_visible(true);
            w.focus_window();
        });

        spawn(async move {
            if let Ok(screens) = Screen::all() {
                if let Some(screen) = screens.first() {
                    if let Ok(image) = screen.capture() {
                        let width = image.width();
                        let height = image.height();
                        let data = image.into_raw();

                        screen_size.set((width, height));

                        let image_info = ImageInfo::new(
                            (width as i32, height as i32),
                            ColorType::RGBA8888,
                            AlphaType::Unpremul,
                            None,
                        );

                        if let Some(skia_img) = images::raster_from_data(
                            &image_info,
                            Data::new_copy(&data),
                            (width * 4) as usize,
                        ) {
                            screenshot_image.set(Some(skia_img));
                        }
                    }
                }
            }
        });
    });

    // 事件处理函数（保持你重构后的逻辑）
    let handle_mouse_down = move |e: MouseEvent| {
        if e.trigger_button == Some(MouseButton::Right) {
            platform.exit();
            return;
        }

        let coords = e.get_element_coordinates();
        let pos = (coords.x as f32 * dpi_scale, coords.y as f32 * dpi_scale);

        if let Some(selection) = *current_selection.read() {
            let toolbar = Toolbar::calculate(&selection, *screen_size.read());

            // 检查工具栏按钮点击
            if let Some(button_index) = toolbar.get_button_index(pos.0, pos.1) {
                match button_index {
                    0 => println!("保存"),
                    1 => println!("复制"),
                    2 => println!("编辑"),
                    3 => println!("分享"),
                    4 => {
                        // current_selection.set(None);
                        app_state.set(AppState::Idle);
                    }
                    _ => {}
                }
                return;
            }

            // 检查调整大小手柄
            if let Some(handle) = geometry::get_resize_handle(pos.0, pos.1, &selection) {
                app_state.set(AppState::Resizing);
                resize_handle.set(Some(handle));
                resize_anchor.set(Some(geometry::get_resize_anchor(handle, &selection)));
                return;
            }

            // 检查拖拽
            if geometry::point_in_rect(pos.0, pos.1, &selection) {
                app_state.set(AppState::Dragging);
                let (left, top, _, _) = selection.bounds();
                drag_offset.set(Some((pos.0 - left, pos.1 - top)));
                return;
            }

            // 如果点击在选择框外面，忽略点击事件（不处理）
            println!("点击在选择框外，忽略点击事件");
            return;
        }

        // 只有在没有现有选择框的情况下才开始新的选择
        app_state.set(AppState::Selecting);
        temp_selection.set(Some(Selection {
            start: pos,
            end: pos,
        }));
        current_selection.set(None);
    };

    // 鼠标移动和释放事件处理（保持你重构后的代码）
    let handle_mouse_move = move |e: MouseEvent| {
        let coords = e.get_element_coordinates();
        let pos = (coords.x as f32 * dpi_scale, coords.y as f32 * dpi_scale);
        mouse_pos.set(pos);

        let current_state = *app_state.read();

        match current_state {
            AppState::Selecting => {
                let temp_sel_opt = *temp_selection.read();
                if let Some(mut selection) = temp_sel_opt {
                    selection.end = pos;
                    temp_selection.set(Some(selection));
                }
            }
            AppState::Dragging => {
                let selection_opt = *current_selection.read();
                let offset_opt = *drag_offset.read();
                let screen_sz = *screen_size.read();

                if let (Some(selection), Some(offset)) = (selection_opt, offset_opt) {
                    let new_selection = Selection {
                        start: (pos.0 - offset.0, pos.1 - offset.1),
                        end: (
                            pos.0 - offset.0 + selection.size().0,
                            pos.1 - offset.1 + selection.size().1,
                        ),
                    };

                    current_selection.set(Some(geometry::constrain_to_screen(
                        new_selection,
                        screen_sz,
                    )));
                }
            }
            AppState::Resizing => {
                // 调整大小逻辑（保持你的完整实现）
                let handle_opt = *resize_handle.read();
                let anchor_opt = *resize_anchor.read();
                let selection_opt = *current_selection.read();
                let screen_sz = *screen_size.read();

                if let (Some(handle), Some(anchor), Some(selection)) =
                    (handle_opt, anchor_opt, selection_opt)
                {
                    let screen_width = screen_sz.0 as f32;
                    let screen_height = screen_sz.1 as f32;

                    let constrained_x = pos.0.max(0.0).min(screen_width);
                    let constrained_y = pos.1.max(0.0).min(screen_height);

                    let (left, top, right, bottom) = selection.bounds();

                    let new_selection = match handle {
                        ResizeHandle::TopLeft => Selection {
                            start: (
                                constrained_x.min(anchor.0 - MIN_SELECTION_SIZE),
                                constrained_y.min(anchor.1 - MIN_SELECTION_SIZE),
                            ),
                            end: anchor,
                        },
                        ResizeHandle::TopRight => Selection {
                            start: (anchor.0, constrained_y.min(anchor.1 - MIN_SELECTION_SIZE)),
                            end: (constrained_x.max(anchor.0 + MIN_SELECTION_SIZE), anchor.1),
                        },
                        ResizeHandle::BottomRight => Selection {
                            start: anchor,
                            end: (
                                constrained_x.max(anchor.0 + MIN_SELECTION_SIZE),
                                constrained_y.max(anchor.1 + MIN_SELECTION_SIZE),
                            ),
                        },
                        ResizeHandle::BottomLeft => Selection {
                            start: (constrained_x.min(anchor.0 - MIN_SELECTION_SIZE), anchor.1),
                            end: (anchor.0, constrained_y.max(anchor.1 + MIN_SELECTION_SIZE)),
                        },
                        ResizeHandle::Top => Selection {
                            start: (left, constrained_y.min(bottom - MIN_SELECTION_SIZE)),
                            end: (right, bottom),
                        },
                        ResizeHandle::Bottom => Selection {
                            start: (left, top),
                            end: (right, constrained_y.max(top + MIN_SELECTION_SIZE)),
                        },
                        ResizeHandle::Left => Selection {
                            start: (constrained_x.min(right - MIN_SELECTION_SIZE), top),
                            end: (right, bottom),
                        },
                        ResizeHandle::Right => Selection {
                            start: (left, top),
                            end: (constrained_x.max(left + MIN_SELECTION_SIZE), bottom),
                        },
                    };

                    current_selection.set(Some(geometry::constrain_to_screen(
                        new_selection,
                        screen_sz,
                    )));
                }
            }
            _ => {}
        }
    };

    let handle_mouse_up = move |_: MouseEvent| {
        let current_state = *app_state.read();

        match current_state {
            AppState::Selecting => {
                let temp_sel = *temp_selection.read();
                if let Some(selection) = temp_sel {
                    current_selection.set(Some(selection));
                }
                temp_selection.set(None);
            }
            AppState::Dragging => {
                drag_offset.set(None);
            }
            AppState::Resizing => {
                resize_handle.set(None);
                resize_anchor.set(None);
            }
            _ => {}
        }

        app_state.set(AppState::Idle);
    };

    // 绘制逻辑（保持你重构后的代码）
    let canvas = use_canvas(move || {
        platform.invalidate_drawing_area(size.peek().area);

        let screenshot = screenshot_image.read().clone();
        let state = *app_state.read();
        let current_sel = *current_selection.read();
        let temp_sel = *temp_selection.read();
        let screen_sz = *screen_size.read();

        let selection = current_sel.or(temp_sel);

        move |ctx| {
            ctx.canvas.clear(Color::TRANSPARENT);

            if let Some(img) = &screenshot {
                let canvas_rect = Rect::from_xywh(0.0, 0.0, ctx.area.width(), ctx.area.height());
                ctx.canvas
                    .draw_image_rect(img, None, canvas_rect, &Paint::default());

                let mut mask_paint = Paint::default();
                mask_paint.set_color(Color::from_argb(160, 0, 0, 0));
                ctx.canvas.draw_rect(canvas_rect, &mask_paint);

                if let Some(sel) = selection {
                    rendering::draw_selection_area(ctx, img, &sel);
                    rendering::draw_selection_border(ctx, &sel, state);

                    if state == AppState::Idle {
                        rendering::draw_handles(ctx, &sel);
                        let toolbar = Toolbar::calculate(&sel, screen_sz);
                        rendering::draw_toolbar(ctx, &toolbar, &sel);
                    }
                }
            }
        }
    });

    // 使用 CursorArea 包装（就像你之前的代码）
    rsx!(
        rect {
            width: "fill",
            height: "fill",
            onmousedown: handle_mouse_down,
            onmousemove: handle_mouse_move,
            onmouseup: handle_mouse_up,
            onglobalkeydown: move |e: KeyboardEvent| {
                if e.key == Key::Escape {
                    platform.exit();
                }
            },
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
