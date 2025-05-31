use freya::core::custom_attributes::CanvasRunnerContext;
use skia_safe::{Color, Paint, PaintStyle, PathEffect, Rect};

use crate::{rendering::selection::draw_selection_handles, types::drawing::DrawingShape};

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
                let boundary_rect =
                    Rect::from_xywh(bounds.0, bounds.1, bounds.2 - bounds.0, bounds.3 - bounds.1);
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
                let boundary_rect =
                    Rect::from_xywh(bounds.0, bounds.1, bounds.2 - bounds.0, bounds.3 - bounds.1);
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
