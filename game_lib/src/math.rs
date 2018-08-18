pub use cgmath;
pub use cgmath::ortho;
pub use cgmath::prelude::*;

const EPSILON: f32 = 0.000_001;

pub type Color = cgmath::Vector4<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;

pub fn is_effectively_zero(x: f32) -> bool {
    f32::abs(x) < EPSILON
}

pub fn is_positive(x: f32) -> bool {
    x > EPSILON
}

//==================================================================================================
// Clamping
//==================================================================================================
//
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

/// A typedef for [`Vec2`] mainly used for representing points.
pub type Point = Vec2;

impl Point {
    /// Clamps a points x and y coordinates to the boundaries of a given rectangle
    ///
    /// # Examples
    /// ```
    /// # use game_lib::math::*;
    ///
    /// let point = Point::new(1.0, 2.5);
    /// let rect = Rect::new(0.0, 0.0, 1.5, 1.5);
    /// assert_eq!(Point::new(1.0, 1.5), point.clamped_in_rect(rect));
    ///
    /// ```
    pub fn clamped_in_rect(self, rect: Rect) -> Point {
        Point::new(
            clamp(self.x, rect.pos.x, rect.pos.x + rect.dim.x),
            clamp(self.y, rect.pos.y, rect.pos.y + rect.dim.x),
        )
    }
}

//==================================================================================================
// Vectors
//==================================================================================================
//
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Neg;
use std::ops::Sub;
use std::ops::SubAssign;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
    }

    pub fn zero() -> Vec2 {
        Vec2 { x: 0.0, y: 0.0 }
    }

    pub fn ones() -> Vec2 {
        Vec2 { x: 1.0, y: 1.0 }
    }

    pub fn unit_x() -> Vec2 {
        Vec2 { x: 1.0, y: 0.0 }
    }

    pub fn unit_y() -> Vec2 {
        Vec2 { x: 0.0, y: 1.0 }
    }

    pub fn distance_squared(a: Vec2, b: Vec2) -> f32 {
        (a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)
    }

    pub fn distance(a: Vec2, b: Vec2) -> f32 {
        f32::sqrt((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y))
    }

    pub fn dot(a: Vec2, b: Vec2) -> f32 {
        a.x * b.x + a.y * b.y
    }
}

// ---------------------------------------------------------------------------------------------
// Negation
//

impl Neg for Vec2 {
    type Output = Vec2;

    fn neg(self) -> Vec2 {
        Vec2 {
            x: -self.x,
            y: -self.y,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Element-wise addition
//
impl Add<Vec2> for Vec2 {
    type Output = Vec2;

    fn add(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl Add<f32> for Vec2 {
    type Output = Vec2;

    fn add(self, scalar: f32) -> Vec2 {
        Vec2 {
            x: self.x + scalar,
            y: self.y + scalar,
        }
    }
}
impl Add<Vec2> for f32 {
    type Output = Vec2;

    fn add(self, vec: Vec2) -> Vec2 {
        Vec2 {
            x: vec.x + self,
            y: vec.y + self,
        }
    }
}

impl AddAssign<Vec2> for Vec2 {
    fn add_assign(&mut self, other: Vec2) {
        *self = Vec2 {
            x: self.x + other.x,
            y: self.y + other.y,
        }
    }
}
impl AddAssign<f32> for Vec2 {
    fn add_assign(&mut self, scalar: f32) {
        *self = Vec2 {
            x: self.x + scalar,
            y: self.y + scalar,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Element-wise subtraction
//
impl Sub<Vec2> for Vec2 {
    type Output = Vec2;

    fn sub(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl Sub<f32> for Vec2 {
    type Output = Vec2;

    fn sub(self, scalar: f32) -> Vec2 {
        Vec2 {
            x: self.x - scalar,
            y: self.y - scalar,
        }
    }
}

impl SubAssign<Vec2> for Vec2 {
    fn sub_assign(&mut self, other: Vec2) {
        *self = Vec2 {
            x: self.x - other.x,
            y: self.y - other.y,
        }
    }
}
impl SubAssign<f32> for Vec2 {
    fn sub_assign(&mut self, scalar: f32) {
        *self = Vec2 {
            x: self.x - scalar,
            y: self.y - scalar,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Element-wise multiplication
//
impl Mul<Vec2> for Vec2 {
    type Output = Vec2;

    fn mul(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x * other.x,
            y: self.y * other.y,
        }
    }
}
impl Mul<f32> for Vec2 {
    type Output = Vec2;

    fn mul(self, scalar: f32) -> Vec2 {
        Vec2 {
            x: self.x * scalar,
            y: self.y * scalar,
        }
    }
}
impl Mul<Vec2> for f32 {
    type Output = Vec2;

    fn mul(self, vec: Vec2) -> Vec2 {
        Vec2 {
            x: vec.x * self,
            y: vec.y * self,
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Element-wise division
//
impl Div<Vec2> for Vec2 {
    type Output = Vec2;

    fn div(self, other: Vec2) -> Vec2 {
        Vec2 {
            x: self.x / other.x,
            y: self.y / other.y,
        }
    }
}
impl Div<f32> for Vec2 {
    type Output = Vec2;

    fn div(self, scalar: f32) -> Vec2 {
        Vec2 {
            x: self.x / scalar,
            y: self.y / scalar,
        }
    }
}

//==================================================================================================
// Matrices
//==================================================================================================
//
pub trait Mat4Helper {
    fn ortho_centered(width: f32, height: f32, near: f32, far: f32) -> Self;
    fn ortho_bottom_left(width: f32, height: f32, near: f32, far: f32) -> Self;
    fn ortho_bottom_left_flipped_y(width: f32, height: f32, near: f32, far: f32) -> Self;
}

impl Mat4Helper for Mat4 {
    fn ortho_centered(width: f32, height: f32, near: f32, far: f32) -> Self {
        cgmath::ortho(
            -0.5 * width,
            0.5 * width,
            -0.5 * height,
            0.5 * height,
            near,
            far,
        )
    }

    fn ortho_bottom_left(width: f32, height: f32, near: f32, far: f32) -> Self {
        cgmath::ortho(0.0, width, 0.0, height, near, far)
    }

    fn ortho_bottom_left_flipped_y(width: f32, height: f32, near: f32, far: f32) -> Self {
        cgmath::ortho(0.0, width, height, 0.0, near, far)
    }
}

//==================================================================================================
// Geometry
//==================================================================================================
//

// TODO(JaSc): Evaluate if it would be better to have the fields of Bounds directly in rect
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub pos: Point,
    pub dim: Vec2,
}

impl Rect {
    pub fn zero() -> Rect {
        Rect {
            pos: Point::zero(),
            dim: Vec2::zero(),
        }
    }
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            pos: Point::new(x, y),
            dim: Vec2::new(width, height),
        }
    }

    pub fn from_width_height(width: f32, height: f32) -> Rect {
        Rect {
            pos: Point::zero(),
            dim: Vec2::new(width, height),
        }
    }

    pub fn from_point(pos: Point, width: f32, height: f32) -> Rect {
        Rect {
            pos,
            dim: Vec2::new(width, height),
        }
    }

    pub fn from_dimension(dim: Vec2) -> Rect {
        Rect {
            pos: Point::zero(),
            dim,
        }
    }

    pub fn from_point_dimension(pos: Point, dim: Vec2) -> Rect {
        Rect { pos, dim }
    }

    pub fn from_corners(bottom_left: Point, top_right: Point) -> Rect {
        Rect {
            pos: Point {
                x: bottom_left.x,
                y: bottom_left.y,
            },
            dim: Vec2 {
                x: top_right.x - bottom_left.x,
                y: top_right.y - bottom_left.y,
            },
        }
    }

    pub fn unit_rect_centered() -> Rect {
        Rect {
            pos: Point { x: -0.5, y: -0.5 },
            dim: Vec2 { x: 1.0, y: 1.0 },
        }
    }

    pub fn width(&self) -> f32 {
        self.dim.x
    }

    pub fn height(&self) -> f32 {
        self.dim.y
    }

    /// Returns the biggest proportionally stretched version of the rectangle that can fit
    /// into `target`.
    pub fn stretched_to_fit(self, target: Rect) -> Rect {
        let source_aspect_ratio = self.width() / self.height();
        let target_aspect_ratio = target.width() / target.height();

        let scale_factor = if source_aspect_ratio < target_aspect_ratio {
            // Target rect is 'wider' than ours -> height is our limit when stretching
            target.height() / self.height()
        } else {
            // Target rect is 'narrower' than ours -> width is our limit when stretching
            target.width() / self.width()
        };

        let stretched_width = self.width() * scale_factor;
        let stretched_height = self.height() * scale_factor;

        Rect::from_point(self.pos, stretched_width, stretched_height)
    }

    /// Returns a version of the rectangle that is centered in `target`.
    pub fn centered_in(self, target: Rect) -> Rect {
        let offset_centered = target.pos + (target.dim - self.dim) / 2.0;

        Rect::from_point_dimension(offset_centered, self.dim)
    }

    pub fn to_bounds(&self) -> Bounds {
        Bounds {
            left: self.pos.x,
            right: self.pos.x + self.dim.x,
            bottom: self.pos.y,
            top: self.pos.y + self.dim.y,
        }
    }

    pub fn to_bounds_centered(&self) -> Bounds {
        Bounds {
            left: self.pos.x - 0.5 * self.dim.x,
            right: self.pos.x + 0.5 * self.dim.x,
            bottom: self.pos.y - 0.5 * self.dim.y,
            top: self.pos.y + 0.5 * self.dim.y,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Bounds {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
}

impl Bounds {
    pub fn new(left: f32, right: f32, bottom: f32, top: f32) -> Bounds {
        Bounds {
            left,
            right,
            bottom,
            top,
        }
    }

    pub fn pos(&self) -> Point {
        Point::new(self.left, self.bottom)
    }

    pub fn pos_centered(&self) -> Point {
        Point::new(
            self.left + 0.5 * (self.right - self.left),
            self.bottom + 0.5 * (self.top - self.bottom),
        )
    }

    pub fn dim(&self) -> Vec2 {
        Vec2::new(self.right - self.left, self.top - self.bottom)
    }

    pub fn from_rect(rect: Rect) -> Bounds {
        rect.to_bounds()
    }

    pub fn from_rect_centered(rect: Rect) -> Bounds {
        rect.to_bounds_centered()
    }

    pub fn to_rect(&self) -> Rect {
        Rect {
            pos: Point::new(self.left, self.right),
            dim: Vec2::new(self.right - self.left, self.top - self.bottom),
        }
    }

    pub fn scaled_from_origin(self, scale: Vec2) -> Bounds {
        debug_assert!(is_positive(scale.x));
        debug_assert!(is_positive(scale.y));
        Bounds {
            left: self.left * scale.x,
            right: self.right * scale.x,
            bottom: self.bottom * scale.y,
            top: self.top * scale.y,
        }
    }

    pub fn to_border_lines(&self) -> [Line; 4] {
        [
            Line {
                start: Point {
                    x: self.left,
                    y: self.bottom,
                },
                end: Point {
                    x: self.right,
                    y: self.bottom,
                },
            },
            Line {
                start: Point {
                    x: self.right,
                    y: self.bottom,
                },
                end: Point {
                    x: self.right,
                    y: self.top,
                },
            },
            Line {
                start: Point {
                    x: self.right,
                    y: self.top,
                },
                end: Point {
                    x: self.left,
                    y: self.top,
                },
            },
            Line {
                start: Point {
                    x: self.left,
                    y: self.top,
                },
                end: Point {
                    x: self.left,
                    y: self.bottom,
                },
            },
        ]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}

impl Line {
    pub fn new(start: Point, end: Point) -> Line {
        Line { start, end }
    }
}

//==================================================================================================
// Camera and coordinate systems
//==================================================================================================
//

/// A point in world-coordinate-space. One 1x1 square in world-space equals to the pixel size
/// on the canvas on a default zoom level.
pub type WorldPoint = Vec2;
/// A point in canvas-coordinate-space. Given in the range
/// `[0, CANVAS_WIDTH - 1]x[0, CANVAS_HEIGHT - 1]`
/// where `(0,0)` is the bottom-left corner
pub type CanvasPoint = Vec2;

/// Same as [`WorldPoint`] only as vector
pub type WorldVec = Vec2;
/// Same as [`CanvasPoint`] only as vector
pub type CanvasVec = Vec2;

impl WorldPoint {
    /// For a given [`WorldPoint`] returns the nearest [`WorldPoint`] that is aligned to the
    /// canvas's pixel grid when drawn.
    ///
    /// For example pixel-snapping the cameras position before drawing prevents pixel-jittering
    /// artifacts on visible objects if the camera is moving at sub-pixel distances.
    pub fn pixel_snapped(self) -> WorldPoint {
        // NOTE: Because OpenGL pixels are drawn from the bottom left we need to floor here
        //       to correctly transform world coordinates to pixels.
        WorldPoint {
            x: f32::floor(self.x),
            y: f32::floor(self.y),
        }
    }
}

/// Camera with its position in the center of its view-rect.
///
/// * `zoom_level > 1.0` : zoomed in
/// * `zoom_level < 1.0` : zoomed out
///
/// # Example: Camera bounds
/// ```
/// # use game_lib::math::*;
///
/// let cam = Camera::new(320, 180, -1.0, 1.0);
///
/// let pos = cam.world_rect.pos;
/// let dim = cam.world_rect.dim / cam.zoom_level;
/// assert_eq!(dim, cam.dim_zoomed());
///
/// let left =   pos.x - 0.5 * dim.x;
/// let right =  pos.x + 0.5 * dim.x;
/// let bottom = pos.y - 0.5 * dim.y;
/// let top =    pos.y + 0.5 * dim.y;
///
/// let bounds = cam.bounds_worldspace();
/// assert_eq!(bounds.left, left);
/// assert_eq!(bounds.right, right);
/// assert_eq!(bounds.bottom, bottom);
/// assert_eq!(bounds.top, top);
/// ```
///
/// # Example: Mouse panning and zooming
///
/// ```
/// # use game_lib::math::*;
///
/// // Canvas and camera setup
/// let canvas_width = 320;
/// let canvas_height = 180;
/// let mut cam = Camera::new(canvas_width, canvas_height, -1.0, 1.0);
///
/// // Current and old mouse state
/// let old_mouse_pos_canvas = Point::new(50.0, 130.0);
/// let new_mouse_pos_canvas = Point::new(60.0, 130.0);
/// let mouse_button_right_pressed = true;
/// let mouse_button_middle_pressed = false;
/// let mouse_wheel_delta = 0;
///
/// // World mouse position and delta
/// let mouse_delta_canvas = new_mouse_pos_canvas - old_mouse_pos_canvas;
/// let old_mouse_pos_world = cam.canvas_to_world(old_mouse_pos_canvas);
/// let new_mouse_pos_world = cam.canvas_to_world(new_mouse_pos_canvas);
/// let _mouse_delta_world = new_mouse_pos_world - old_mouse_pos_world;
///
/// // Pan camera
/// if mouse_button_right_pressed {
///     cam.pan(mouse_delta_canvas);
/// }
/// // Reset zoom
/// if mouse_button_middle_pressed {
///     cam.zoom_to_world_point(new_mouse_pos_world, 1.0);
/// }
/// // Zoom in or out by factors of two
/// if mouse_wheel_delta > 0 {
///     // Magnify up till 8x
///     let new_zoom_level = f32::min(cam.zoom_level * 2.0, 8.0);
///     cam.zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
/// } else if mouse_wheel_delta < 0 {
///     // Minify down till 1/8
///     let new_zoom_level = f32::max(cam.zoom_level / 2.0, 1.0 / 8.0);
///     cam.zoom_to_world_point(new_mouse_pos_world, new_zoom_level);
/// }
///
/// // Get project-view-matrix from cam and use it for drawing
/// let transform = cam.proj_view_matrix();
///
/// // ..
/// ```
pub struct Camera {
    pub world_rect: Rect,
    pub zoom_level: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(canvas_width: i32, canvas_height: i32, z_near: f32, z_far: f32) -> Camera {
        Camera::with_position(
            WorldPoint::zero(),
            canvas_width,
            canvas_height,
            z_near,
            z_far,
        )
    }

    pub fn with_position(
        pos: WorldPoint,
        canvas_width: i32,
        canvas_height: i32,
        z_near: f32,
        z_far: f32,
    ) -> Camera {
        Camera {
            world_rect: Rect::new(pos.x, pos.y, canvas_width as f32, canvas_height as f32),
            zoom_level: 1.0,
            z_near,
            z_far,
        }
    }

    pub fn pos(&self) -> WorldPoint {
        self.world_rect.pos
    }

    pub fn dim_zoomed(&self) -> WorldVec {
        self.world_rect.dim / self.zoom_level
    }

    /// Converts a [`CanvasPoint`] into a [`WorldPoint`]
    pub fn canvas_to_world(&self, point: CanvasPoint) -> WorldPoint {
        (point - 0.5 * self.world_rect.dim) / self.zoom_level + self.pos().pixel_snapped()
    }

    /// Converts a [`WorldPoint`] into a [`CanvasPoint`]
    pub fn world_to_canvas(&self, point: WorldPoint) -> CanvasPoint {
        (point - self.pos().pixel_snapped()) * self.zoom_level + 0.5 * self.world_rect.dim
    }

    /// Zooms the camera to or away from a given world point.
    ///
    /// * `new_zoom_level > 1.0` -> magnify
    /// * `new_zoom_level < 1.0` -> minify
    pub fn zoom_to_world_point(&mut self, world_point: WorldPoint, new_zoom_level: f32) {
        let old_zoom_level = self.zoom_level;
        self.zoom_level = new_zoom_level;
        self.world_rect.pos =
            (self.world_rect.pos - world_point) * (old_zoom_level / new_zoom_level) + world_point;
    }

    /// Pans the camera using cursor movement distance on the canvas
    pub fn pan(&mut self, canvas_move_distance: CanvasVec) {
        self.world_rect.pos -= canvas_move_distance / self.zoom_level;
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&self) -> Mat4 {
        use cgmath::Vector3;

        let translation = -self.world_rect.pos.pixel_snapped();
        let view_mat = Mat4::from_nonuniform_scale(self.zoom_level, self.zoom_level, 1.0)
            * Mat4::from_translation(Vector3::new(translation.x, translation.y, 0.0));

        let proj_mat = Mat4::ortho_centered(
            self.world_rect.dim.x,
            self.world_rect.dim.y,
            self.z_near,
            self.z_far,
        );
        proj_mat * view_mat
    }

    /// Returns the [`Bounds`] of the cameras view in world-space
    pub fn bounds_worldspace(&self) -> Bounds {
        let world_rect_zoomed = Rect::from_point_dimension(self.world_rect.pos, self.dim_zoomed());
        world_rect_zoomed.to_bounds_centered()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converting_between_canvas_and_world_coordinates_and_back() {
        let cam = Camera::new(100, 100, -1.0, 1.0);

        let canvas_point = CanvasPoint::new(0.75, -0.23);
        assert!(is_effectively_zero(CanvasPoint::distance(
            canvas_point,
            cam.world_to_canvas(cam.canvas_to_world(canvas_point))
        )));

        let world_point = WorldPoint::new(-12.3, 134.0);
        assert!(is_effectively_zero(WorldPoint::distance(
            world_point,
            cam.canvas_to_world(cam.world_to_canvas(world_point))
        )));
    }
}
