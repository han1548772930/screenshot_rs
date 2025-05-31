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
                x.with_fullscreen(Some(Fullscreen::Borderless(None)))
                    .with_resizable(false)
                // .with_window_level(WindowLevel::AlwaysOnTop)
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
    Drawing,
    EditingShape,
    ResizingShape, // 新增：调整图形大小
    Idle,
}
// 在现有的枚举和结构体之后添加绘图相关的定义

#[derive(Debug, Clone, Copy, PartialEq)]
enum DrawingTool {
    None,
    Rectangle,
    Circle,
    Arrow,
    Brush,
}

#[derive(Debug, Clone)]
enum DrawingShape {
    Rectangle {
        start: (f32, f32),
        end: (f32, f32),
        color: Color,
        stroke_width: f32,
    },
    Circle {
        center: (f32, f32),
        radius: f32,
        color: Color,
        stroke_width: f32,
    },
    Arrow {
        start: (f32, f32),
        end: (f32, f32),
        color: Color,
        stroke_width: f32,
    },
    BrushStroke {
        points: Vec<(f32, f32)>,
        color: Color,
        stroke_width: f32,
    },
}
impl DrawingShape {
    // 添加调整大小手柄检测
    fn get_resize_handle(&self, x: f32, y: f32) -> Option<ResizeHandle> {
        let (left, top, right, bottom) = self.bounds();
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        match self {
            DrawingShape::Circle { .. } => {
                // 圆形只检查四个角手柄
                if (x - left).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE {
                    return Some(ResizeHandle::TopLeft);
                }
                if (x - right).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE
                {
                    return Some(ResizeHandle::TopRight);
                }
                if (x - right).abs() <= HANDLE_DETECT_SIZE
                    && (y - bottom).abs() <= HANDLE_DETECT_SIZE
                {
                    return Some(ResizeHandle::BottomRight);
                }
                if (x - left).abs() <= HANDLE_DETECT_SIZE
                    && (y - bottom).abs() <= HANDLE_DETECT_SIZE
                {
                    return Some(ResizeHandle::BottomLeft);
                }
                None
            }
            _ => {
                // 其他图形保持原有的全部8个手柄检测
                // 检查角手柄（优先级最高）
                if (x - left).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE {
                    return Some(ResizeHandle::TopLeft);
                }
                if (x - right).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE
                {
                    return Some(ResizeHandle::TopRight);
                }
                if (x - right).abs() <= HANDLE_DETECT_SIZE
                    && (y - bottom).abs() <= HANDLE_DETECT_SIZE
                {
                    return Some(ResizeHandle::BottomRight);
                }
                if (x - left).abs() <= HANDLE_DETECT_SIZE
                    && (y - bottom).abs() <= HANDLE_DETECT_SIZE
                {
                    return Some(ResizeHandle::BottomLeft);
                }

                // 检查边手柄（确保不与角手柄重叠）
                // 上边中间
                if (x - center_x).abs() <= HANDLE_DETECT_SIZE
                    && (y - top).abs() <= HANDLE_DETECT_SIZE
                {
                    // 确保不在角手柄范围内
                    if (x - left).abs() > HANDLE_DETECT_SIZE
                        && (x - right).abs() > HANDLE_DETECT_SIZE
                    {
                        return Some(ResizeHandle::Top);
                    }
                }
                // 右边中间
                if (x - right).abs() <= HANDLE_DETECT_SIZE
                    && (y - center_y).abs() <= HANDLE_DETECT_SIZE
                {
                    // 确保不在角手柄范围内
                    if (y - top).abs() > HANDLE_DETECT_SIZE
                        && (y - bottom).abs() > HANDLE_DETECT_SIZE
                    {
                        return Some(ResizeHandle::Right);
                    }
                }
                // 下边中间
                if (x - center_x).abs() <= HANDLE_DETECT_SIZE
                    && (y - bottom).abs() <= HANDLE_DETECT_SIZE
                {
                    // 确保不在角手柄范围内
                    if (x - left).abs() > HANDLE_DETECT_SIZE
                        && (x - right).abs() > HANDLE_DETECT_SIZE
                    {
                        return Some(ResizeHandle::Bottom);
                    }
                }
                // 左边中间
                if (x - left).abs() <= HANDLE_DETECT_SIZE
                    && (y - center_y).abs() <= HANDLE_DETECT_SIZE
                {
                    // 确保不在角手柄范围内
                    if (y - top).abs() > HANDLE_DETECT_SIZE
                        && (y - bottom).abs() > HANDLE_DETECT_SIZE
                    {
                        return Some(ResizeHandle::Left);
                    }
                }

                None
            }
        }
    }

    // 获取调整大小锚点
    fn get_resize_anchor(&self, handle: ResizeHandle) -> (f32, f32) {
        let (left, top, right, bottom) = self.bounds();

        match self {
            DrawingShape::Circle { .. } => {
                // 圆形只有四个角手柄，锚点是对角
                match handle {
                    ResizeHandle::TopLeft => (right, bottom),
                    ResizeHandle::TopRight => (left, bottom),
                    ResizeHandle::BottomRight => (left, top),
                    ResizeHandle::BottomLeft => (right, top),
                    _ => (left, top), // 圆形不应该有其他手柄，但提供默认值
                }
            }
            _ => {
                // 其他图形保持原有逻辑
                match handle {
                    ResizeHandle::TopLeft => (right, bottom),
                    ResizeHandle::TopRight => (left, bottom),
                    ResizeHandle::BottomRight => (left, top),
                    ResizeHandle::BottomLeft => (right, top),
                    ResizeHandle::Top => (left, bottom),
                    ResizeHandle::Bottom => (left, top),
                    ResizeHandle::Left => (right, top),
                    ResizeHandle::Right => (left, top),
                }
            }
        }
    }

    // 修改调整大小方法，限制在选择区域内
    fn resize_constrained(
        &mut self,
        new_bounds: (f32, f32, f32, f32),
        selection_bounds: (f32, f32, f32, f32),
    ) {
        let (new_left, new_top, new_right, new_bottom) = new_bounds;
        let (sel_left, sel_top, sel_right, sel_bottom) = selection_bounds;

        // 限制在选择区域内
        let constrained_left = new_left.max(sel_left).min(sel_right - 10.0);
        let constrained_top = new_top.max(sel_top).min(sel_bottom - 10.0);
        let constrained_right = new_right.min(sel_right).max(sel_left + 10.0);
        let constrained_bottom = new_bottom.min(sel_bottom).max(sel_top + 10.0);

        match self {
            DrawingShape::Rectangle { start, end, .. } => {
                *start = (constrained_left, constrained_top);
                *end = (constrained_right, constrained_bottom);
            }
            DrawingShape::Circle { center, radius, .. } => {
                // 确保边界是有效的
                if constrained_left >= constrained_right || constrained_top >= constrained_bottom {
                    return;
                }

                // 计算新的边界框尺寸
                let new_width = constrained_right - constrained_left;
                let new_height = constrained_bottom - constrained_top;

                // 取较小的尺寸作为直径，确保圆形保持圆形
                let diameter = new_width.min(new_height);
                let new_radius = diameter / 2.0;

                // 最小半径限制
                let min_radius = 5.0;
                if new_radius < min_radius {
                    return;
                }

                // 计算新的中心点，确保圆形在边界框中居中
                let new_center_x = (constrained_left + constrained_right) / 2.0;
                let new_center_y = (constrained_top + constrained_bottom) / 2.0;

                // 检查新的圆是否完全在选择区域内
                let circle_left = new_center_x - new_radius;
                let circle_right = new_center_x + new_radius;
                let circle_top = new_center_y - new_radius;
                let circle_bottom = new_center_y + new_radius;

                // 如果圆超出选择区域，重新计算一个安全的半径
                if circle_left < sel_left
                    || circle_right > sel_right
                    || circle_top < sel_top
                    || circle_bottom > sel_bottom
                {
                    // 计算各个方向的最大允许半径
                    let max_radius_x = (new_center_x - sel_left).min(sel_right - new_center_x);
                    let max_radius_y = (new_center_y - sel_top).min(sel_bottom - new_center_y);
                    let safe_radius = max_radius_x.min(max_radius_y).max(min_radius);

                    *radius = safe_radius;
                } else {
                    *radius = new_radius;
                }

                *center = (new_center_x, new_center_y);
            }
            DrawingShape::Arrow { start, end, .. } => {
                *start = (constrained_left, constrained_top);
                *end = (constrained_right, constrained_bottom);
            }
            DrawingShape::BrushStroke { .. } => {
                // 画笔笔迹不支持调整大小
            }
        }
    }

    // 限制位置在选择区域内
    fn constrain_to_selection(&mut self, selection_bounds: (f32, f32, f32, f32)) {
        let (sel_left, sel_top, sel_right, sel_bottom) = selection_bounds;
        let (left, top, right, bottom) = self.bounds();

        let width = right - left;
        let height = bottom - top;

        // 计算需要移动的距离
        let mut dx = 0.0;
        let mut dy = 0.0;

        if left < sel_left {
            dx = sel_left - left;
        } else if right > sel_right {
            dx = sel_right - right;
        }

        if top < sel_top {
            dy = sel_top - top;
        } else if bottom > sel_bottom {
            dy = sel_bottom - bottom;
        }

        if dx != 0.0 || dy != 0.0 {
            self.translate(dx, dy);
        }
    }
    fn bounds(&self) -> (f32, f32, f32, f32) {
        match self {
            DrawingShape::Rectangle { start, end, .. } => {
                let left = start.0.min(end.0);
                let right = start.0.max(end.0);
                let top = start.1.min(end.1);
                let bottom = start.1.max(end.1);
                (left, top, right, bottom)
            }
            DrawingShape::Circle { center, radius, .. } => (
                center.0 - radius,
                center.1 - radius,
                center.0 + radius,
                center.1 + radius,
            ),
            DrawingShape::Arrow { start, end, .. } => {
                let left = start.0.min(end.0);
                let right = start.0.max(end.0);
                let top = start.1.min(end.1);
                let bottom = start.1.max(end.1);
                (left, top, right, bottom)
            }
            DrawingShape::BrushStroke { points, .. } => {
                if points.is_empty() {
                    return (0.0, 0.0, 0.0, 0.0);
                }
                let mut min_x = points[0].0;
                let mut max_x = points[0].0;
                let mut min_y = points[0].1;
                let mut max_y = points[0].1;

                for &(x, y) in points {
                    min_x = min_x.min(x);
                    max_x = max_x.max(x);
                    min_y = min_y.min(y);
                    max_y = max_y.max(y);
                }
                (min_x, min_y, max_x, max_y)
            }
        }
    }

    fn contains_point(&self, x: f32, y: f32) -> bool {
        match self {
            DrawingShape::Rectangle { .. } => {
                let (left, top, right, bottom) = self.bounds();
                x >= left && x <= right && y >= top && y <= bottom
            }
            DrawingShape::Circle { center, radius, .. } => {
                // 只检查是否在圆形内部，不是边界矩形
                let dx = x - center.0;
                let dy = y - center.1;
                let distance = (dx * dx + dy * dy).sqrt();
                distance <= *radius
            }
            DrawingShape::Arrow { .. } => {
                let (left, top, right, bottom) = self.bounds();
                x >= left && x <= right && y >= top && y <= bottom
            }
            DrawingShape::BrushStroke { .. } => {
                let (left, top, right, bottom) = self.bounds();
                x >= left && x <= right && y >= top && y <= bottom
            }
        }
    }

    fn translate(&mut self, dx: f32, dy: f32) {
        match self {
            DrawingShape::Rectangle { start, end, .. } => {
                start.0 += dx;
                start.1 += dy;
                end.0 += dx;
                end.1 += dy;
            }
            DrawingShape::Circle { center, .. } => {
                center.0 += dx;
                center.1 += dy;
            }
            DrawingShape::Arrow { start, end, .. } => {
                start.0 += dx;
                start.1 += dy;
                end.0 += dx;
                end.1 += dy;
            }
            DrawingShape::BrushStroke { points, .. } => {
                for point in points {
                    point.0 += dx;
                    point.1 += dy;
                }
            }
        }
    }
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

        // 检查角手柄（优先级最高）
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

        // 检查边手柄（确保不与角手柄重叠）
        // 上边中间
        if (x - center_x).abs() <= HANDLE_DETECT_SIZE && (y - top).abs() <= HANDLE_DETECT_SIZE {
            // 确保不在角手柄范围内
            if (x - left).abs() > HANDLE_DETECT_SIZE && (x - right).abs() > HANDLE_DETECT_SIZE {
                return Some(ResizeHandle::Top);
            }
        }
        // 右边中间
        if (x - right).abs() <= HANDLE_DETECT_SIZE && (y - center_y).abs() <= HANDLE_DETECT_SIZE {
            // 确保不在角手柄范围内
            if (y - top).abs() > HANDLE_DETECT_SIZE && (y - bottom).abs() > HANDLE_DETECT_SIZE {
                return Some(ResizeHandle::Right);
            }
        }
        // 下边中间
        if (x - center_x).abs() <= HANDLE_DETECT_SIZE && (y - bottom).abs() <= HANDLE_DETECT_SIZE {
            // 确保不在角手柄范围内
            if (x - left).abs() > HANDLE_DETECT_SIZE && (x - right).abs() > HANDLE_DETECT_SIZE {
                return Some(ResizeHandle::Bottom);
            }
        }
        // 左边中间
        if (x - left).abs() <= HANDLE_DETECT_SIZE && (y - center_y).abs() <= HANDLE_DETECT_SIZE {
            // 确保不在角手柄范围内
            if (y - top).abs() > HANDLE_DETECT_SIZE && (y - bottom).abs() > HANDLE_DETECT_SIZE {
                return Some(ResizeHandle::Left);
            }
        }

        None
    }

    pub fn get_resize_anchor(handle: ResizeHandle, selection: &Selection) -> (f32, f32) {
        let (left, top, right, bottom) = selection.bounds();
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        match handle {
            ResizeHandle::TopLeft => (right, bottom),
            ResizeHandle::TopRight => (left, bottom),
            ResizeHandle::BottomRight => (left, top),
            ResizeHandle::BottomLeft => (right, top),
            ResizeHandle::Top => (center_x, bottom),
            ResizeHandle::Bottom => (center_x, top),
            ResizeHandle::Left => (right, center_y),
            ResizeHandle::Right => (left, center_y),
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
                x - HANDLE_SIZE / 2.0,
                y - HANDLE_SIZE / 2.0,
                HANDLE_SIZE,
                HANDLE_SIZE,
            );
            ctx.canvas.draw_rect(rect, &handle_paint);
            ctx.canvas.draw_rect(rect, &border_paint);
        }
    }

    pub fn draw_toolbar(
        ctx: &mut CanvasRunnerContext,
        toolbar: &Toolbar,
        _selection: &Selection,
        mouse_pos: (f32, f32),
    ) {
        let buttons = ["rectangle", "circle", "arrow", "brush", "close"];

        for (i, icon_type) in buttons.iter().enumerate() {
            let button_x = toolbar.x + i as f32 * (BUTTON_WIDTH + BUTTON_SPACING);
            let button_rect = Rect::from_xywh(button_x, toolbar.y, BUTTON_WIDTH, BUTTON_HEIGHT);

            // 检查鼠标是否在当前按钮上
            let is_hovered = mouse_pos.0 >= button_x
                && mouse_pos.0 <= button_x + BUTTON_WIDTH
                && mouse_pos.1 >= toolbar.y
                && mouse_pos.1 <= toolbar.y + BUTTON_HEIGHT;

            // 根据 hover 状态设置不同的颜色
            let mut button_paint = Paint::default();
            if is_hovered {
                // Hover 状态：更亮的背景色
                button_paint.set_color(Color::from_argb(240, 80, 80, 80));
            } else {
                // 正常状态
                button_paint.set_color(Color::from_argb(220, 45, 45, 45));
            }
            button_paint.set_anti_alias(true);

            let mut border_paint = Paint::default();
            if is_hovered {
                // Hover 状态：更亮的边框
                border_paint.set_color(Color::from_rgb(220, 220, 220));
            } else {
                border_paint.set_color(Color::from_rgb(180, 180, 180));
            }
            border_paint.set_style(PaintStyle::Stroke);
            border_paint.set_stroke_width(1.0);
            border_paint.set_anti_alias(true);

            // 绘制按钮背景
            ctx.canvas
                .draw_round_rect(button_rect, 4.0, 4.0, &button_paint);
            ctx.canvas
                .draw_round_rect(button_rect, 4.0, 4.0, &border_paint);

            // 绘制图标
            draw_icon(
                ctx,
                icon_type,
                button_x + BUTTON_WIDTH / 2.0,
                toolbar.y + BUTTON_HEIGHT / 2.0,
                is_hovered,
            );
        }
    }

    fn draw_icon(
        ctx: &mut CanvasRunnerContext,
        icon_type: &str,
        center_x: f32,
        center_y: f32,
        is_hovered: bool,
    ) {
        let mut paint = Paint::default();
        // Hover 时图标颜色更亮
        if is_hovered {
            paint.set_color(Color::from_rgb(255, 255, 255));
        } else {
            paint.set_color(Color::from_rgb(200, 200, 200));
        }
        paint.set_stroke_width(2.0);
        paint.set_anti_alias(true);

        let size = 8.0;

        match icon_type {
            "rectangle" => {
                // 画框图标
                paint.set_style(PaintStyle::Stroke);
                let rect = Rect::from_xywh(
                    center_x - size,
                    center_y - size * 0.7,
                    size * 2.0,
                    size * 1.4,
                );
                ctx.canvas.draw_rect(rect, &paint);
            }
            "circle" => {
                // 画圆图标
                paint.set_style(PaintStyle::Stroke);
                ctx.canvas.draw_circle((center_x, center_y), size, &paint);
            }
            "arrow" => {
                // 箭头图标
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_cap(skia_safe::PaintCap::Round);

                // 箭头主体
                ctx.canvas.draw_line(
                    (center_x - size, center_y + size * 0.5),
                    (center_x + size, center_y - size * 0.5),
                    &paint,
                );

                // 箭头头部
                ctx.canvas.draw_line(
                    (center_x + size, center_y - size * 0.5),
                    (center_x + size * 0.3, center_y - size * 0.8),
                    &paint,
                );
                ctx.canvas.draw_line(
                    (center_x + size, center_y - size * 0.5),
                    (center_x + size * 0.3, center_y - size * 0.2),
                    &paint,
                );
            }
            "brush" => {
                // 自由画笔图标
                paint.set_style(PaintStyle::Fill);

                // 画笔笔身
                let brush_rect = Rect::from_xywh(
                    center_x - size * 0.3,
                    center_y - size,
                    size * 0.6,
                    size * 1.5,
                );
                ctx.canvas.draw_rect(brush_rect, &paint);

                // 画笔头
                if is_hovered {
                    paint.set_color(Color::from_rgb(255, 200, 150));
                } else {
                    paint.set_color(Color::from_rgb(200, 150, 100));
                }
                let brush_tip = Rect::from_xywh(
                    center_x - size * 0.2,
                    center_y + size * 0.3,
                    size * 0.4,
                    size * 0.5,
                );
                ctx.canvas.draw_rect(brush_tip, &paint);
            }
            "close" => {
                // 关闭图标（X）
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_cap(skia_safe::PaintCap::Round);
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
    pub fn draw_shape(ctx: &mut CanvasRunnerContext, shape: &DrawingShape, is_selected: bool) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        match shape {
            DrawingShape::Rectangle {
                start,
                end,
                color,
                stroke_width,
            } => {
                paint.set_color(*color);
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(*stroke_width);

                let left = start.0.min(end.0);
                let top = start.1.min(end.1);
                let width = (end.0 - start.0).abs();
                let height = (end.1 - start.1).abs();

                let rect = Rect::from_xywh(left, top, width, height);
                ctx.canvas.draw_rect(rect, &paint);

                if is_selected {
                    draw_selection_handles(ctx, shape);
                }
            }
            DrawingShape::Circle {
                center,
                radius,
                color,
                stroke_width,
            } => {
                paint.set_color(*color);
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(*stroke_width);

                // 绘制圆形
                ctx.canvas.draw_circle(*center, *radius, &paint);

                // 在绘制或选中状态时显示边界框虚线
                if is_selected {
                    // 绘制边界框虚线
                    let mut boundary_paint = Paint::default();
                    boundary_paint.set_color(Color::from_rgb(128, 128, 128));
                    boundary_paint.set_style(PaintStyle::Stroke);
                    boundary_paint.set_stroke_width(1.0);
                    boundary_paint.set_anti_alias(true);

                    // 添加虚线效果
                    if let Some(dash_effect) = PathEffect::dash(&[5.0, 5.0], 0.0) {
                        boundary_paint.set_path_effect(dash_effect);
                    }

                    // 绘制圆的边界矩形
                    let boundary_rect = Rect::from_xywh(
                        center.0 - radius,
                        center.1 - radius,
                        radius * 2.0,
                        radius * 2.0,
                    );
                    ctx.canvas.draw_rect(boundary_rect, &boundary_paint);

                    // 绘制调整大小手柄
                    draw_selection_handles(ctx, shape);
                }
            }
            DrawingShape::Arrow {
                start,
                end,
                color,
                stroke_width,
            } => {
                paint.set_color(*color);
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(*stroke_width);
                paint.set_stroke_cap(skia_safe::PaintCap::Round);

                // 绘制箭头主体
                ctx.canvas.draw_line(*start, *end, &paint);

                // 计算箭头头部
                let dx = end.0 - start.0;
                let dy = end.1 - start.1;
                let angle = dy.atan2(dx);
                let arrow_length = 15.0;
                let arrow_angle = 0.5;

                let arrow_point1 = (
                    end.0 - arrow_length * (angle - arrow_angle).cos(),
                    end.1 - arrow_length * (angle - arrow_angle).sin(),
                );
                let arrow_point2 = (
                    end.0 - arrow_length * (angle + arrow_angle).cos(),
                    end.1 - arrow_length * (angle + arrow_angle).sin(),
                );

                ctx.canvas.draw_line(*end, arrow_point1, &paint);
                ctx.canvas.draw_line(*end, arrow_point2, &paint);

                if is_selected {
                    // 绘制边界框虚线
                    let mut boundary_paint = Paint::default();
                    boundary_paint.set_color(Color::from_rgb(128, 128, 128));
                    boundary_paint.set_style(PaintStyle::Stroke);
                    boundary_paint.set_stroke_width(1.0);
                    boundary_paint.set_anti_alias(true);

                    if let Some(dash_effect) = PathEffect::dash(&[5.0, 5.0], 0.0) {
                        boundary_paint.set_path_effect(dash_effect);
                    }

                    let bounds = shape.bounds();
                    let boundary_rect = Rect::from_xywh(
                        bounds.0,
                        bounds.1,
                        bounds.2 - bounds.0,
                        bounds.3 - bounds.1,
                    );
                    ctx.canvas.draw_rect(boundary_rect, &boundary_paint);

                    draw_selection_handles(ctx, shape);
                }
            }
            DrawingShape::BrushStroke {
                points,
                color,
                stroke_width,
            } => {
                if points.len() < 2 {
                    return;
                }

                paint.set_color(*color);
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(*stroke_width);
                paint.set_stroke_cap(skia_safe::PaintCap::Round);
                paint.set_stroke_join(skia_safe::PaintJoin::Round);

                for window in points.windows(2) {
                    ctx.canvas.draw_line(window[0], window[1], &paint);
                }

                if is_selected {
                    // 绘制边界框虚线
                    let mut boundary_paint = Paint::default();
                    boundary_paint.set_color(Color::from_rgb(128, 128, 128));
                    boundary_paint.set_style(PaintStyle::Stroke);
                    boundary_paint.set_stroke_width(1.0);
                    boundary_paint.set_anti_alias(true);

                    if let Some(dash_effect) = PathEffect::dash(&[5.0, 5.0], 0.0) {
                        boundary_paint.set_path_effect(dash_effect);
                    }

                    let bounds = shape.bounds();
                    let boundary_rect = Rect::from_xywh(
                        bounds.0,
                        bounds.1,
                        bounds.2 - bounds.0,
                        bounds.3 - bounds.1,
                    );
                    ctx.canvas.draw_rect(boundary_rect, &boundary_paint);

                    // 画笔笔迹不支持调整大小，只显示边界框
                }
            }
        }
    }
    pub fn draw_drawing_shape(ctx: &mut CanvasRunnerContext, shape: &DrawingShape) {
        let mut paint = Paint::default();
        paint.set_anti_alias(true);

        match shape {
            DrawingShape::Circle {
                center,
                radius,
                color,
                stroke_width,
            } => {
                paint.set_color(*color);
                paint.set_style(PaintStyle::Stroke);
                paint.set_stroke_width(*stroke_width);

                // 绘制圆形
                ctx.canvas.draw_circle(*center, *radius, &paint);

                // 绘制边界框虚线（绘制中状态）
                let mut boundary_paint = Paint::default();
                boundary_paint.set_color(Color::from_rgb(0, 255, 255)); // 绘制中用青色
                boundary_paint.set_style(PaintStyle::Stroke);
                boundary_paint.set_stroke_width(1.0);
                boundary_paint.set_anti_alias(true);

                // 添加虚线效果
                if let Some(dash_effect) = PathEffect::dash(&[3.0, 3.0], 0.0) {
                    boundary_paint.set_path_effect(dash_effect);
                }

                // 绘制圆的边界矩形
                let boundary_rect = Rect::from_xywh(
                    center.0 - radius,
                    center.1 - radius,
                    radius * 2.0,
                    radius * 2.0,
                );
                ctx.canvas.draw_rect(boundary_rect, &boundary_paint);
            }
            _ => {
                // 其他图形正常绘制
                draw_shape(ctx, shape, false);
            }
        }
    }

    // 新增函数：绘制选择手柄
    fn draw_selection_handles(ctx: &mut CanvasRunnerContext, shape: &DrawingShape) {
        let (left, top, right, bottom) = shape.bounds();
        let center_x = (left + right) / 2.0;
        let center_y = (top + bottom) / 2.0;

        let mut handle_paint = Paint::default();
        handle_paint.set_color(Color::from_rgb(255, 255, 255));
        handle_paint.set_anti_alias(true);

        let mut border_paint = Paint::default();
        border_paint.set_color(Color::from_rgb(128, 128, 128));
        border_paint.set_style(PaintStyle::Stroke);
        border_paint.set_stroke_width(1.0);
        border_paint.set_anti_alias(true);

        match shape {
            DrawingShape::Rectangle { .. } | DrawingShape::Arrow { .. } => {
                // 矩形和箭头显示全部8个手柄
                let handles = [
                    (left, top),        // 左上
                    (center_x, top),    // 上中
                    (right, top),       // 右上
                    (right, center_y),  // 右中
                    (right, bottom),    // 右下
                    (center_x, bottom), // 下中
                    (left, bottom),     // 左下
                    (left, center_y),   // 左中
                ];

                for (x, y) in handles {
                    let rect = Rect::from_xywh(
                        x - HANDLE_SIZE / 2.0,
                        y - HANDLE_SIZE / 2.0,
                        HANDLE_SIZE,
                        HANDLE_SIZE,
                    );
                    ctx.canvas.draw_rect(rect, &handle_paint);
                    ctx.canvas.draw_rect(rect, &border_paint);
                }
            }
            DrawingShape::Circle { .. } => {
                // 圆形只显示4个角的手柄
                let handles = [
                    (left, top),     // 左上
                    (right, top),    // 右上
                    (right, bottom), // 右下
                    (left, bottom),  // 左下
                ];

                for (x, y) in handles {
                    let rect = Rect::from_xywh(
                        x - HANDLE_SIZE / 2.0,
                        y - HANDLE_SIZE / 2.0,
                        HANDLE_SIZE,
                        HANDLE_SIZE,
                    );
                    ctx.canvas.draw_rect(rect, &handle_paint);
                    ctx.canvas.draw_rect(rect, &border_paint);
                }
            }
            DrawingShape::BrushStroke { .. } => {
                // 画笔笔迹不支持调整大小，不绘制手柄
            }
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
    let mut last_cursor = use_signal(|| CursorIcon::Default);

    // 新增绘图状态
    let mut current_tool = use_signal(|| DrawingTool::None);
    let mut drawing_shapes = use_signal::<Vec<DrawingShape>>(|| Vec::new());
    let mut current_drawing = use_signal::<Option<DrawingShape>>(|| None);
    let mut selected_shape_index = use_signal::<Option<usize>>(|| None);
    let mut shape_drag_offset = use_signal::<Option<(f32, f32)>>(|| None);
    let mut shape_resize_handle = use_signal::<Option<ResizeHandle>>(|| None);
    let mut shape_resize_anchor = use_signal::<Option<(f32, f32)>>(|| None);

    let (reference, size) = use_node_signal();

    // 修改光标检测函数中的优先级
    let mut get_cursor_icon = move || -> CursorIcon {
        let (x, y) = *mouse_pos.read();
        let current_state = *app_state.read();

        let new_cursor = match current_state {
            AppState::Selecting => CursorIcon::Crosshair,
            AppState::Dragging => CursorIcon::Move,
            AppState::Resizing => {
                if let Some(handle) = *resize_handle.read() {
                    resize_handle_to_cursor(handle)
                } else {
                    CursorIcon::Default
                }
            }
            AppState::Drawing => {
                let tool = *current_tool.read();
                match tool {
                    DrawingTool::Rectangle => CursorIcon::Crosshair,
                    DrawingTool::Circle => CursorIcon::Crosshair,
                    DrawingTool::Arrow => CursorIcon::Crosshair,
                    DrawingTool::Brush => CursorIcon::Crosshair,
                    DrawingTool::None => CursorIcon::Default,
                }
            }
            AppState::EditingShape => CursorIcon::Move,
            AppState::ResizingShape => {
                if let Some(handle) = *shape_resize_handle.read() {
                    resize_handle_to_cursor(handle)
                } else {
                    CursorIcon::Default
                }
            }
            AppState::Idle => {
                if let Some(selection) = *current_selection.read() {
                    let toolbar = Toolbar::calculate(&selection, *screen_size.read());

                    // 1. 优先检查工具栏
                    if toolbar.contains_point(x, y) {
                        CursorIcon::Pointer
                    }
                    // 2. 然后检查选择框调整大小手柄（始终优先）
                    else if let Some(handle) = geometry::get_resize_handle(x, y, &selection) {
                        resize_handle_to_cursor(handle)
                    }
                    // 3. 检查选择框内部
                    else if geometry::point_in_rect(x, y, &selection) {
                        // 优先检查是否有选中的图形的调整大小手柄
                        if let Some(selected_idx) = *selected_shape_index.read() {
                            let shapes = drawing_shapes.read();
                            if let Some(shape) = shapes.get(selected_idx) {
                                if let Some(handle) = shape.get_resize_handle(x, y) {
                                    return resize_handle_to_cursor(handle);
                                }
                            }
                        }

                        // 然后检查是否在任何图形上（但不是调整手柄）
                        let shapes = drawing_shapes.read();
                        for shape in shapes.iter().rev() {
                            if shape.contains_point(x, y) {
                                // 再次检查确保不是在调整手柄上
                                if shape.get_resize_handle(x, y).is_none() {
                                    return CursorIcon::Pointer;
                                }
                            }
                        }

                        // 检查是否有绘图工具选中
                        let tool = *current_tool.read();
                        if tool != DrawingTool::None {
                            // 有绘图工具时，显示绘图光标
                            match tool {
                                DrawingTool::Rectangle => CursorIcon::Crosshair,
                                DrawingTool::Circle => CursorIcon::Crosshair,
                                DrawingTool::Arrow => CursorIcon::Crosshair,
                                DrawingTool::Brush => CursorIcon::Crosshair,
                                DrawingTool::None => CursorIcon::Move,
                            }
                        } else {
                            // 没有绘图工具时，显示移动光标
                            CursorIcon::Move
                        }
                    } else {
                        // 在选择框外部 - 始终显示禁止光标
                        CursorIcon::NotAllowed
                    }
                } else {
                    CursorIcon::Default
                }
            }
        };

        if new_cursor != *last_cursor.read() {
            last_cursor.set(new_cursor);
        }

        new_cursor
    };

    // 初始化逻辑（保持不变）
    use_effect(move || {
        platform.with_window(|w| {
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

    // 同时修改鼠标按下事件处理，确保图形调整手柄优先级正确
    let handle_mouse_down = move |e: MouseEvent| {
        if e.trigger_button == Some(MouseButton::Right) {
            platform.exit();
            return;
        }

        let coords = e.get_element_coordinates();
        let pos = (coords.x as f32 * dpi_scale, coords.y as f32 * dpi_scale);

        // 先读取当前选择状态，避免借用冲突
        let current_sel = *current_selection.read();

        if let Some(selection) = current_sel {
            let toolbar = Toolbar::calculate(&selection, *screen_size.read());

            // 1. 检查工具栏按钮点击
            if let Some(button_index) = toolbar.get_button_index(pos.0, pos.1) {
                match button_index {
                    0 => {
                        println!("画框工具");
                        current_tool.set(DrawingTool::Rectangle);
                        selected_shape_index.set(None);
                    }
                    1 => {
                        println!("画圆工具");
                        current_tool.set(DrawingTool::Circle);
                        selected_shape_index.set(None);
                    }
                    2 => {
                        println!("画箭头工具");
                        current_tool.set(DrawingTool::Arrow);
                        selected_shape_index.set(None);
                    }
                    3 => {
                        println!("自由画笔工具");
                        current_tool.set(DrawingTool::Brush);
                        selected_shape_index.set(None);
                    }
                    4 => {
                        platform.exit();
                        app_state.set(AppState::Idle);
                    }
                    _ => {}
                }
                return;
            }

            // 2. 优先检查选择框的调整大小手柄（不管是否有绘图工具）
            if let Some(handle) = geometry::get_resize_handle(pos.0, pos.1, &selection) {
                app_state.set(AppState::Resizing);
                resize_handle.set(Some(handle));
                resize_anchor.set(Some(geometry::get_resize_anchor(handle, &selection)));
                return;
            }

            // 3. 检查是否点击了选择框内部
            if geometry::point_in_rect(pos.0, pos.1, &selection) {
                let tool = *current_tool.read();

                // 绝对优先检查选中图形的调整手柄（不管是否有绘图工具或其他条件）
                if let Some(selected_idx) = *selected_shape_index.read() {
                    let shapes = drawing_shapes.read();
                    if let Some(shape) = shapes.get(selected_idx) {
                        if let Some(handle) = shape.get_resize_handle(pos.0, pos.1) {
                            // drop(shapes); // 释放借用
                            app_state.set(AppState::ResizingShape);
                            shape_resize_handle.set(Some(handle));
                            shape_resize_anchor.set(Some(shape.get_resize_anchor(handle)));
                            return;
                        }
                    }
                    drop(shapes);
                }

                // 然后检查是否点击了任何图形的调整手柄（不管是否选中）
                let shapes = drawing_shapes.read();
                for (i, shape) in shapes.iter().enumerate().rev() {
                    if let Some(handle) = shape.get_resize_handle(pos.0, pos.1) {
                        // drop(shapes); // 释放借用
                        selected_shape_index.set(Some(i));
                        app_state.set(AppState::ResizingShape);
                        shape_resize_handle.set(Some(handle));
                        shape_resize_anchor.set(Some(shape.get_resize_anchor(handle)));
                        return;
                    }
                }
                drop(shapes);

                // 如果有绘图工具选中
                if tool != DrawingTool::None {
                    // 检查是否点击了已有的图形本身（不是调整手柄）
                    let shapes = drawing_shapes.read();
                    for (i, shape) in shapes.iter().enumerate().rev() {
                        if shape.contains_point(pos.0, pos.1) {
                            drop(shapes); // 释放借用
                            // 点击了图形本身，进入编辑模式
                            selected_shape_index.set(Some(i));
                            app_state.set(AppState::EditingShape);
                            let shapes = drawing_shapes.read();
                            if let Some(shape) = shapes.get(i) {
                                let (left, top, _, _) = shape.bounds();
                                shape_drag_offset.set(Some((pos.0 - left, pos.1 - top)));
                            }
                            return;
                        }
                    }
                    drop(shapes);

                    // 如果没有点击到图形，开始新的绘制
                    app_state.set(AppState::Drawing);
                    let default_color = Color::from_rgb(255, 0, 0);
                    let default_stroke = 1.0;

                    let new_shape = match tool {
                        DrawingTool::Rectangle => DrawingShape::Rectangle {
                            start: pos,
                            end: pos,
                            color: default_color,
                            stroke_width: default_stroke,
                        },
                        DrawingTool::Circle => DrawingShape::Circle {
                            center: pos,
                            radius: 0.0,
                            color: default_color,
                            stroke_width: default_stroke,
                        },
                        DrawingTool::Arrow => DrawingShape::Arrow {
                            start: pos,
                            end: pos,
                            color: default_color,
                            stroke_width: default_stroke,
                        },
                        DrawingTool::Brush => DrawingShape::BrushStroke {
                            points: vec![pos],
                            color: default_color,
                            stroke_width: default_stroke,
                        },
                        DrawingTool::None => return,
                    };

                    current_drawing.set(Some(new_shape));
                    return;
                } else {
                    // 没有绘图工具选中，检查是否点击了图形本身
                    let shapes = drawing_shapes.read();
                    for (i, shape) in shapes.iter().enumerate().rev() {
                        if shape.contains_point(pos.0, pos.1) {
                            drop(shapes); // 释放借用
                            // 点击了图形本身，进入编辑模式
                            selected_shape_index.set(Some(i));
                            app_state.set(AppState::EditingShape);
                            let shapes = drawing_shapes.read();
                            if let Some(shape) = shapes.get(i) {
                                let (left, top, _, _) = shape.bounds();
                                shape_drag_offset.set(Some((pos.0 - left, pos.1 - top)));
                            }
                            return;
                        }
                    }
                    drop(shapes);

                    // 没有点击到图形，开始拖拽选择框
                    app_state.set(AppState::Dragging);
                    let (left, top, _, _) = selection.bounds();
                    drag_offset.set(Some((pos.0 - left, pos.1 - top)));
                    return;
                }
            } else {
                // 点击在选择框外部 - 直接忽略，不做任何操作
                return;
            }
        } else {
            // 没有选择框时，允许新建选择
            app_state.set(AppState::Selecting);
            temp_selection.set(Some(Selection {
                start: pos,
                end: pos,
            }));
            current_selection.set(None);
            current_tool.set(DrawingTool::None);
            selected_shape_index.set(None);
        }
    };

    // 鼠标移动和释放事件处理（保持你重构后的代码）

    let handle_mouse_move = move |e: MouseEvent| {
        let coords = e.get_element_coordinates();
        let pos = (coords.x as f32 * dpi_scale, coords.y as f32 * dpi_scale);
        mouse_pos.set(pos);
        let current_state = *app_state.read();

        match current_state {
            AppState::Drawing => {
                if let Some(selection) = *current_selection.read() {
                    let selection_bounds = selection.bounds();

                    // 限制绘制位置在选择区域内
                    let constrained_pos = (
                        pos.0.max(selection_bounds.0).min(selection_bounds.2),
                        pos.1.max(selection_bounds.1).min(selection_bounds.3),
                    );

                    let current_shape = current_drawing.read().clone();
                    if let Some(mut shape) = current_shape {
                        match &mut shape {
                            DrawingShape::Rectangle { end, .. }
                            | DrawingShape::Arrow { end, .. } => {
                                *end = constrained_pos;
                            }
                            DrawingShape::Circle { center, radius, .. } => {
                                let dx = constrained_pos.0 - center.0;
                                let dy = constrained_pos.1 - center.1;
                                let new_radius = (dx * dx + dy * dy).sqrt();

                                // 限制圆不超出选择区域
                                let max_radius = (center.0 - selection_bounds.0)
                                    .min(selection_bounds.2 - center.0)
                                    .min(center.1 - selection_bounds.1)
                                    .min(selection_bounds.3 - center.1);

                                *radius = new_radius.min(max_radius);
                            }
                            DrawingShape::BrushStroke { points, .. } => {
                                // 只有在选择区域内才添加点
                                if geometry::point_in_rect(pos.0, pos.1, &selection) {
                                    points.push(pos);
                                }
                            }
                        }
                        current_drawing.set(Some(shape));
                    }
                }
            }
            AppState::ResizingShape => {
                if let (Some(index), Some(handle), Some(anchor), Some(selection)) = (
                    *selected_shape_index.read(),
                    *shape_resize_handle.read(),
                    *shape_resize_anchor.read(),
                    *current_selection.read(),
                ) {
                    let mut shapes = drawing_shapes.read().clone();
                    if let Some(shape) = shapes.get_mut(index) {
                        let selection_bounds = selection.bounds();

                        // 限制鼠标位置在选择区域内
                        let constrained_pos = (
                            pos.0.max(selection_bounds.0).min(selection_bounds.2),
                            pos.1.max(selection_bounds.1).min(selection_bounds.3),
                        );

                        let (left, top, right, bottom) = shape.bounds();

                        // 根据不同的手柄计算新的边界
                        let new_bounds = match handle {
                            ResizeHandle::TopLeft => {
                                (constrained_pos.0, constrained_pos.1, anchor.0, anchor.1)
                            }
                            ResizeHandle::TopRight => {
                                (anchor.0, constrained_pos.1, constrained_pos.0, anchor.1)
                            }
                            ResizeHandle::BottomRight => {
                                (anchor.0, anchor.1, constrained_pos.0, constrained_pos.1)
                            }
                            ResizeHandle::BottomLeft => {
                                (constrained_pos.0, anchor.1, anchor.0, constrained_pos.1)
                            }
                            ResizeHandle::Top => (left, constrained_pos.1, right, anchor.1),
                            ResizeHandle::Bottom => (left, anchor.1, right, constrained_pos.1),
                            ResizeHandle::Left => (constrained_pos.0, top, anchor.0, bottom),
                            ResizeHandle::Right => (anchor.0, top, constrained_pos.0, bottom),
                        };

                        // 确保新边界是有效的（左小于右，上小于下）
                        let (mut new_left, mut new_top, mut new_right, mut new_bottom) = new_bounds;

                        if new_left > new_right {
                            std::mem::swap(&mut new_left, &mut new_right);
                        }
                        if new_top > new_bottom {
                            std::mem::swap(&mut new_top, &mut new_bottom);
                        }

                        // 确保最小尺寸
                        let min_size = 10.0;
                        if new_right - new_left < min_size {
                            if handle == ResizeHandle::Left
                                || handle == ResizeHandle::TopLeft
                                || handle == ResizeHandle::BottomLeft
                            {
                                new_left = new_right - min_size;
                            } else {
                                new_right = new_left + min_size;
                            }
                        }
                        if new_bottom - new_top < min_size {
                            if handle == ResizeHandle::Top
                                || handle == ResizeHandle::TopLeft
                                || handle == ResizeHandle::TopRight
                            {
                                new_top = new_bottom - min_size;
                            } else {
                                new_bottom = new_top + min_size;
                            }
                        }

                        shape.resize_constrained(
                            (new_left, new_top, new_right, new_bottom),
                            selection_bounds,
                        );
                        drawing_shapes.set(shapes);
                    }
                }
            }
            AppState::EditingShape => {
                let selected_idx = *selected_shape_index.read();
                let offset = *shape_drag_offset.read();

                if let (Some(index), Some(offset), Some(selection)) =
                    (selected_idx, offset, *current_selection.read())
                {
                    let mut shapes = drawing_shapes.read().clone();
                    if let Some(shape) = shapes.get_mut(index) {
                        let new_x = pos.0 - offset.0;
                        let new_y = pos.1 - offset.1;
                        let (old_left, old_top, _, _) = shape.bounds();
                        let dx = new_x - old_left;
                        let dy = new_y - old_top;
                        shape.translate(dx, dy);

                        // 限制在选择区域内
                        shape.constrain_to_selection(selection.bounds());
                        drawing_shapes.set(shapes);
                    }
                }
            }
            AppState::Selecting => {
                let temp_sel = *temp_selection.read();
                if let Some(mut selection) = temp_sel {
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

    // 修改鼠标释放处理
    let handle_mouse_up = move |_: MouseEvent| {
        let current_state = *app_state.read();

        match current_state {
            AppState::Drawing => {
                let current_shape = current_drawing.read().clone();
                if let Some(shape) = current_shape {
                    let mut shapes = drawing_shapes.read().clone();
                    shapes.push(shape);
                    drawing_shapes.set(shapes);
                    current_drawing.set(None);
                }
                app_state.set(AppState::Idle);
            }
            AppState::ResizingShape => {
                shape_resize_handle.set(None);
                shape_resize_anchor.set(None);
                app_state.set(AppState::Idle);
            }

            AppState::EditingShape => {
                shape_drag_offset.set(None);
                app_state.set(AppState::Idle);
            }
            AppState::Selecting => {
                let temp_sel = *temp_selection.read();
                if let Some(selection) = temp_sel {
                    current_selection.set(Some(selection));
                }
                temp_selection.set(None);
                app_state.set(AppState::Idle);
            }
            AppState::Dragging => {
                drag_offset.set(None);
                app_state.set(AppState::Idle);
            }
            AppState::Resizing => {
                resize_handle.set(None);
                resize_anchor.set(None);
                app_state.set(AppState::Idle);
            }
            _ => {}
        }
    };

    // 修改 canvas 部分
    let canvas = use_canvas(move || {
        platform.invalidate_drawing_area(size.peek().area);

        let screenshot = screenshot_image.read().clone();
        let state = *app_state.read();
        let current_sel = *current_selection.read();
        let temp_sel = *temp_selection.read();
        let screen_sz = *screen_size.read();
        let mouse_position = *mouse_pos.read();
        let shapes = drawing_shapes.read().clone();
        let current_draw = current_drawing.read().clone();
        let selected_idx = *selected_shape_index.read();
        let tool = *current_tool.read();

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
                        // 只有在没有选择绘图工具时才显示选择框的调整手柄
                        if tool == DrawingTool::None {
                            rendering::draw_handles(ctx, &sel);
                        }

                        let toolbar = Toolbar::calculate(&sel, screen_sz);
                        rendering::draw_toolbar(ctx, &toolbar, &sel, mouse_position);
                    }
                }

                // 绘制所有已完成的图形
                for (i, shape) in shapes.iter().enumerate() {
                    let is_selected = selected_idx == Some(i);
                    rendering::draw_shape(ctx, shape, is_selected);
                }

                // 绘制正在绘制的图形（使用特殊的绘制函数）
                if let Some(shape) = &current_draw {
                    rendering::draw_drawing_shape(ctx, shape);
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
