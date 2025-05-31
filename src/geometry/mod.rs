
use crate::{
    constants::constants::HANDLE_DETECT_SIZE,
    types::{app_state::ResizeHandle, ui::Selection},
};

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
