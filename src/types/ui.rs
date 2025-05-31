use crate::constants::constants::{
    BUTTON_HEIGHT, BUTTON_SPACING, BUTTON_WIDTH, SCREEN_MARGIN, TOOLBAR_MARGIN, TOTAL_BUTTONS,
};

#[derive(Debug, Clone, Copy)]
pub struct Selection {
    pub start: (f32, f32),
    pub end: (f32, f32),
}

pub struct Toolbar {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Selection {
    pub fn bounds(&self) -> (f32, f32, f32, f32) {
        let left = self.start.0.min(self.end.0);
        let right = self.start.0.max(self.end.0);
        let top = self.start.1.min(self.end.1);
        let bottom = self.start.1.max(self.end.1);
        (left, top, right, bottom)
    }

    pub fn center(&self) -> (f32, f32) {
        let (left, top, right, bottom) = self.bounds();
        ((left + right) / 2.0, (top + bottom) / 2.0)
    }

    pub fn size(&self) -> (f32, f32) {
        let (left, top, right, bottom) = self.bounds();
        (right - left, bottom - top)
    }
}
impl Toolbar {
  pub  fn calculate(selection: &Selection, screen_size: (u32, u32)) -> Self {
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

   pub fn contains_point(&self, x: f32, y: f32) -> bool {
        x >= self.x && x <= self.x + self.width && y >= self.y && y <= self.y + self.height
    }

  pub  fn get_button_index(&self, x: f32, y: f32) -> Option<usize> {
        if !self.contains_point(x, y) {
            return None;
        }
        let relative_x = x - self.x;
        let index = (relative_x / (BUTTON_WIDTH + BUTTON_SPACING)).floor() as usize;
        if index < 5 { Some(index) } else { None }
    }
}
