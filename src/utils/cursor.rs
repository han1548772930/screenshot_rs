use freya::prelude::{Readable, Signal, Writable};
use winit::window::CursorIcon;

use crate::types::app_state::ResizeHandle;

pub fn resize_handle_to_cursor(handle: ResizeHandle) -> CursorIcon {
    match handle {
        ResizeHandle::TopLeft | ResizeHandle::BottomRight => CursorIcon::NwResize,
        ResizeHandle::TopRight | ResizeHandle::BottomLeft => CursorIcon::NeResize,
        ResizeHandle::Top | ResizeHandle::Bottom => CursorIcon::NsResize,
        ResizeHandle::Left | ResizeHandle::Right => CursorIcon::EwResize,
    }
}

use crate::geometry::{get_resize_handle, point_in_rect};
use crate::types::{
    app_state::AppState,
    drawing::{DrawingShape, DrawingTool},
    ui::{Selection, Toolbar},
};

/// 光标管理器
pub struct CursorManager;

impl CursorManager {
    pub fn get_cursor_icon_with_cache(
        mouse_pos: (f32, f32),
        app_state: AppState,
        current_selection: Option<Selection>,
        current_tool: DrawingTool,
        drawing_shapes: &[DrawingShape],
        selected_shape_index: Option<usize>,
        resize_handle: Option<ResizeHandle>,
        shape_resize_handle: Option<ResizeHandle>,
        screen_size: (u32, u32),
        last_cursor: &mut Signal<CursorIcon>, // 传入可变引用
    ) -> CursorIcon {
        let new_cursor = Self::get_cursor_icon(
            mouse_pos,
            app_state,
            current_selection,
            current_tool,
            drawing_shapes,
            selected_shape_index,
            resize_handle,
            shape_resize_handle,
            screen_size,
        );

        // 只有当光标改变时才更新状态
        if new_cursor != *last_cursor.read() {
            last_cursor.set(new_cursor);
        }

        new_cursor
    }

    /// 获取当前应该显示的光标图标
    fn get_cursor_icon(
        mouse_pos: (f32, f32),
        app_state: AppState,
        current_selection: Option<Selection>,
        current_tool: DrawingTool,
        drawing_shapes: &[DrawingShape],
        selected_shape_index: Option<usize>,
        resize_handle: Option<ResizeHandle>,
        shape_resize_handle: Option<ResizeHandle>,
        screen_size: (u32, u32),
    ) -> CursorIcon {
        let (x, y) = mouse_pos;

        match app_state {
            AppState::Selecting => CursorIcon::Crosshair,
            AppState::Dragging => CursorIcon::Move,
            AppState::Resizing => {
                if let Some(handle) = resize_handle {
                    resize_handle_to_cursor(handle)
                } else {
                    CursorIcon::Default
                }
            }
            AppState::Drawing => Self::get_drawing_cursor(current_tool),
            AppState::EditingShape => CursorIcon::Move,
            AppState::ResizingShape => {
                if let Some(handle) = shape_resize_handle {
                    resize_handle_to_cursor(handle)
                } else {
                    CursorIcon::Default
                }
            }
            AppState::Idle => Self::get_idle_cursor(
                x,
                y,
                current_selection,
                current_tool,
                drawing_shapes,
                selected_shape_index,
                screen_size,
            ),
        }
    }

    /// 获取绘制时的光标
    fn get_drawing_cursor(tool: DrawingTool) -> CursorIcon {
        match tool {
            DrawingTool::Rectangle => CursorIcon::Crosshair,
            DrawingTool::Circle => CursorIcon::Crosshair,
            DrawingTool::Arrow => CursorIcon::Crosshair,
            DrawingTool::Brush => CursorIcon::Crosshair,
            DrawingTool::None => CursorIcon::Default,
        }
    }

    /// 获取空闲状态时的光标
    fn get_idle_cursor(
        x: f32,
        y: f32,
        current_selection: Option<Selection>,
        current_tool: DrawingTool,
        drawing_shapes: &[DrawingShape],
        selected_shape_index: Option<usize>,
        screen_size: (u32, u32),
    ) -> CursorIcon {
        if let Some(selection) = current_selection {
            let toolbar = Toolbar::calculate(&selection, screen_size);

            // 1. 优先检查工具栏
            if toolbar.contains_point(x, y) {
                CursorIcon::Pointer
            }
            // 2. 只有在没有绘图工具时才检查选择框调整手柄
            else if current_tool == DrawingTool::None
                && get_resize_handle(x, y, &selection).is_some()
            {
                let handle = get_resize_handle(x, y, &selection).unwrap();
                resize_handle_to_cursor(handle)
            }
            // 3. 检查选择框内部
            else if point_in_rect(x, y, &selection) {
                Self::get_selection_area_cursor(
                    x,
                    y,
                    current_tool,
                    drawing_shapes,
                    selected_shape_index,
                )
            } else {
                // 在选择框外部 - 始终显示禁止光标
                CursorIcon::NotAllowed
            }
        } else {
            CursorIcon::Default
        }
    }

    /// 获取选择区域内的光标
    fn get_selection_area_cursor(
        x: f32,
        y: f32,
        current_tool: DrawingTool,
        drawing_shapes: &[DrawingShape],
        selected_shape_index: Option<usize>,
    ) -> CursorIcon {
        // 优先检查是否有选中的图形的调整大小手柄
        if let Some(selected_idx) = selected_shape_index {
            if let Some(shape) = drawing_shapes.get(selected_idx) {
                if let Some(handle) = shape.get_resize_handle(x, y) {
                    return resize_handle_to_cursor(handle);
                }
            }
        }

        // 然后检查是否在任何图形上（但不是调整手柄）
        for shape in drawing_shapes.iter().rev() {
            if shape.contains_point(x, y) {
                // 再次检查确保不是在调整手柄上
                if shape.get_resize_handle(x, y).is_none() {
                    return CursorIcon::Pointer;
                }
            }
        }

        // 检查是否有绘图工具选中
        if current_tool != DrawingTool::None {
            // 有绘图工具时，显示绘图光标
            Self::get_drawing_cursor(current_tool)
        } else {
            // 没有绘图工具时，显示移动光标
            CursorIcon::Move
        }
    }
}
