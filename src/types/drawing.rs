use skia_safe::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DrawingTool {
    None,
    Rectangle,
    Circle,
    Arrow,
    Brush,
}

#[derive(Debug, Clone)]
pub enum DrawingShape {
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