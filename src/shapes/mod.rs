use crate::{
    constants::constants::HANDLE_DETECT_SIZE,
    types::{app_state::ResizeHandle, drawing::DrawingShape},
};

impl DrawingShape {
    // 添加调整大小手柄检测
    pub fn get_resize_handle(&self, x: f32, y: f32) -> Option<ResizeHandle> {
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
    pub fn get_resize_anchor(&self, handle: ResizeHandle) -> (f32, f32) {
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
    pub fn resize_constrained(
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
    pub fn constrain_to_selection(&mut self, selection_bounds: (f32, f32, f32, f32)) {
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
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
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

    pub fn contains_point(&self, x: f32, y: f32) -> bool {
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

    pub fn translate(&mut self, dx: f32, dy: f32) {
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
