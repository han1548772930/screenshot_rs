
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeHandle {
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
pub enum AppState {
    Selecting,
    Dragging,
    Resizing,
    Drawing,
    EditingShape,
    ResizingShape, // 新增：调整图形大小
    Idle,
}