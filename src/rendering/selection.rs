use freya::core::custom_attributes::CanvasRunnerContext;
use skia_safe::{
    AlphaType, Color, ColorType, Data, Image as SkiaImage, ImageInfo, Paint, PaintStyle,
    PathEffect, Rect, canvas::SrcRectConstraint, images,
};

use crate::{constants::constants::HANDLE_SIZE, types::{app_state::AppState, drawing::DrawingShape, ui::Selection}};


pub fn draw_selection_area(ctx: &mut CanvasRunnerContext, img: &SkiaImage, selection: &Selection) {
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

// 新增函数：绘制选择手柄
pub fn draw_selection_handles(ctx: &mut CanvasRunnerContext, shape: &DrawingShape) {
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
