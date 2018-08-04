pub use cgmath;
pub use cgmath::ortho;
pub use cgmath::prelude::*;

pub type Point = cgmath::Point2<f32>;
pub type Vec2 = cgmath::Vector2<f32>;
pub type Color = cgmath::Vector4<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;

/// Clamps a given f32 `val` into interval \[`min`, `max`\]
pub fn clamp(val: f32, min: f32, max: f32) -> f32 {
    debug_assert!(min <= max);
    f32::max(min, f32::min(val, max))
}

/// Clamps a given integer `val` into interval \[`min`, `max`\]
pub fn clamp_integer(val: i32, min: i32, max: i32) -> i32 {
    debug_assert!(min <= max);
    i32::max(min, i32::min(val, max))
}

#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rect {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            x,
            y,
            width,
            height,
        }
    }

    pub fn from_dimension(width: f32, height: f32) -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width,
            height,
        }
    }

    pub fn from_corners(bottom_left: Point, top_right: Point) -> Rect {
        Rect {
            x: bottom_left.x,
            y: bottom_left.y,
            width: top_right.x - bottom_left.x,
            height: top_right.y - bottom_left.y,
        }
    }

    pub fn unit_rect_centered() -> Rect {
        Rect {
            x: -0.5,
            y: -0.5,
            width: 1.0,
            height: 1.0,
        }
    }

    /// Returns the biggest proportionally stretched version of the rectangle that can fit
    /// into `target`.
    pub fn stretched_to_fit(self, target: Rect) -> Rect {
        let source_aspect_ratio = self.width / self.height;
        let target_aspect_ratio = target.width / target.height;

        let scale_factor = if source_aspect_ratio < target_aspect_ratio {
            // Target rect is 'wider' than ours -> height is our limit when stretching
            target.height / self.height
        } else {
            // Target rect is 'narrower' than ours -> width is our limit when stretching
            target.width / self.width
        };

        let stretched_width = self.width * scale_factor;
        let stretched_height = self.height * scale_factor;

        Rect {
            x: self.x,
            y: self.x,
            width: stretched_width,
            height: stretched_height,
        }
    }

    /// Returns a version of the rectangle that is centered in `target`.
    pub fn centered_in(self, target: Rect) -> Rect {
        let x_offset_centered = target.x + (target.width - self.width) / 2.0;
        let y_offset_centered = target.y + (target.height - self.height) / 2.0;

        Rect {
            x: x_offset_centered,
            y: y_offset_centered,
            width: self.width,
            height: self.height,
        }
    }

    pub fn to_pos(&self) -> Point {
        Point::new(self.x, self.y)
    }

    pub fn to_dim(&self) -> Vec2 {
        Vec2::new(self.width, self.height)
    }
}

/// Clamps a points x and y coordinates to the boundaries of a given rectangle
///
/// # Examples
/// ```
/// let point = Point::new(1.0, 2.5);
/// let rect = Rect::new(0.0, 0.0, 1.5, 1.5);
/// assert_eq!(Point::new(1.0, 1.5), clamp_point_in_rect(point, rect));
///
/// ```
pub fn clamp_point_in_rect(point: Point, rect: Rect) -> Point {
    Point::new(
        clamp(point.x, rect.x, rect.x + rect.width),
        clamp(point.y, rect.y, rect.y + rect.height),
    )
}
