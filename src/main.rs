#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use display_info::DisplayInfo;
use freya::prelude::*;

use freya_test::{
    constants::constants::MIN_SELECTION_SIZE,
    geometry::{constrain_to_screen, get_resize_anchor, get_resize_handle, point_in_rect},
    rendering::{
        selection::{draw_handles, draw_selection_area, draw_selection_border},
        shapes::{draw_drawing_shape, draw_shape},
        toolbar::draw_toolbar,
    },
    types::{
        app_state::{AppState, ResizeHandle},
        drawing::{DrawingShape, DrawingTool},
        ui::{Selection, Toolbar},
    },
    utils::cursor::CursorManager,
};
use screenshots::Screen;
use skia_safe::{
    AlphaType, Color, ColorType, Data, Image as SkiaImage, ImageInfo, Paint, Rect, images,
};

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
                x.with_fullscreen(Some(Fullscreen::Borderless(None)))
                    .with_resizable(false)
                // .with_window_level(WindowLevel::AlwaysOnTop)
            }),
    );
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

            // 🔧 修复：检查是否有绘图工具选中
            let tool = *current_tool.read();

            // 2. 只有在没有绘图工具时才检查选择框的调整大小手柄
            if tool == DrawingTool::None {
                if let Some(handle) = get_resize_handle(pos.0, pos.1, &selection) {
                    app_state.set(AppState::Resizing);
                    resize_handle.set(Some(handle));
                    resize_anchor.set(Some(get_resize_anchor(handle, &selection)));
                    return;
                }
            }

            // 3. 检查是否点击了选择框内部
            if point_in_rect(pos.0, pos.1, &selection) {
                // 绝对优先检查选中图形的调整手柄
                if let Some(selected_idx) = *selected_shape_index.read() {
                    let shapes = drawing_shapes.read();
                    if let Some(shape) = shapes.get(selected_idx) {
                        if let Some(handle) = shape.get_resize_handle(pos.0, pos.1) {
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
                            drop(shapes);
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
                            drop(shapes);
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

                    // 🔧 优化1：避免频繁克隆，使用 with_mut
                    current_drawing.with_mut(|current_shape_opt| {
                        if let Some(shape) = current_shape_opt {
                            match shape {
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
                                    // 🔧 优化2：画笔优化 - 减少点的数量和频率
                                    if point_in_rect(pos.0, pos.1, &selection) {
                                        // 只有在距离上一个点足够远时才添加新点
                                        let should_add_point =
                                            if let Some(last_point) = points.last() {
                                                let dx = pos.0 - last_point.0;
                                                let dy = pos.1 - last_point.1;
                                                let distance = (dx * dx + dy * dy).sqrt();
                                                distance > 2.0 // 最小距离阈值
                                            } else {
                                                true
                                            };

                                        if should_add_point {
                                            points.push(pos);

                                            // 🔧 优化3：限制点的总数，防止内存无限增长
                                            const MAX_BRUSH_POINTS: usize = 1000;
                                            if points.len() > MAX_BRUSH_POINTS {
                                                // 移除最早的一些点，保持流畅度
                                                points.drain(0..100);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    });
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
                    drawing_shapes.with_mut(|shapes| {
                        if let Some(shape) = shapes.get_mut(index) {
                            let new_x = pos.0 - offset.0;
                            let new_y = pos.1 - offset.1;
                            let (old_left, old_top, _, _) = shape.bounds();
                            let dx = new_x - old_left;
                            let dy = new_y - old_top;

                            // 🔧 优化2：只有当移动距离足够大时才更新
                            if dx.abs() > 0.5 || dy.abs() > 0.5 {
                                shape.translate(dx, dy);
                                // 限制在选择区域内
                                shape.constrain_to_selection(selection.bounds());
                            }
                        }
                    });
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

                    current_selection.set(Some(constrain_to_screen(new_selection, screen_sz)));
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

                    current_selection.set(Some(constrain_to_screen(new_selection, screen_sz)));
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
                    draw_selection_area(ctx, img, &sel);
                    draw_selection_border(ctx, &sel, state);

                    if state == AppState::Idle {
                        // 只有在没有选择绘图工具时才显示选择框的调整手柄
                        if tool == DrawingTool::None {
                            draw_handles(ctx, &sel);
                        }

                        let toolbar = Toolbar::calculate(&sel, screen_sz);
                        draw_toolbar(ctx, &toolbar, &sel, mouse_position);
                    }
                }

                // 绘制所有已完成的图形
                for (i, shape) in shapes.iter().enumerate() {
                    let is_selected = selected_idx == Some(i);
                    draw_shape(ctx, shape, is_selected);
                }

                // 绘制正在绘制的图形（使用特殊的绘制函数）
                if let Some(shape) = &current_draw {
                    draw_drawing_shape(ctx, shape);
                }
            }
        }
    });

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
                icon:CursorManager:: get_cursor_icon_with_cache(
                    *mouse_pos.read(),
                    *app_state.read(),
                    current_selection.read().clone(),
                    *current_tool.read(),
                    &drawing_shapes.read(),
                    *selected_shape_index.read(),
                    resize_handle.read().clone(),
                    shape_resize_handle.read().clone(),
                    *screen_size.read(),
                    &mut last_cursor,
                ),
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
