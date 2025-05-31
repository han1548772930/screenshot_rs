use freya::core::custom_attributes::CanvasRunnerContext;
use skia_safe::{Color, Paint, PaintStyle, Rect};

use crate::{
    constants::constants::{BUTTON_HEIGHT, BUTTON_SPACING, BUTTON_WIDTH},
    types::ui::{Selection, Toolbar},
};

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
