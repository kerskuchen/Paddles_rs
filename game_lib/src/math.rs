pub use cgmath;
pub use cgmath::ortho;
pub use cgmath::prelude::*;

use cgmath::Vector3;

pub type Point = Vec2;
pub type WorldPoint = Vec2;
pub type ScreenPoint = Vec2;
pub type Color = cgmath::Vector4<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn zero() -> Vec2 {
        Vec2 { x: 0.0, y: 0.0 }
    }
    pub fn new(x: f32, y: f32) -> Vec2 {
        Vec2 { x, y }
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
// Rect
//==================================================================================================
//
#[derive(Debug, Clone, Copy)]
pub struct Rect {
    pub pos: Point,
    pub dim: Vec2,
}

#[derive(Debug, Clone, Copy)]
pub struct Bounds {
    pub left: f32,
    pub right: f32,
    pub bottom: f32,
    pub top: f32,
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
            dim: dim,
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

    pub fn bounds(&self) -> Bounds {
        Bounds {
            left: self.pos.x,
            right: self.pos.x + self.dim.x,
            bottom: self.pos.y,
            top: self.pos.y + self.dim.y,
        }
    }

    pub fn bounds_centered(&self) -> Bounds {
        Bounds {
            left: self.pos.x - 0.5 * self.dim.x,
            right: self.pos.x + 0.5 * self.dim.x,
            bottom: self.pos.y - 0.5 * self.dim.y,
            top: self.pos.y + 0.5 * self.dim.y,
        }
    }
}

//==================================================================================================
// Camera and coordinate systems
//==================================================================================================
//

pub const PIXELS_PER_UNIT: f32 = 16 as f32;
pub const PIXEL_SIZE: f32 = 1.0 / PIXELS_PER_UNIT;

impl WorldPoint {
    /// For a given [`WorldPoint`] returns the nearest [`WorldPoint`] that is aligned to the
    /// screen's pixel grid when drawn.
    ///
    /// Pixel-snapping the cameras position for example before drawing prevents pixel-jittering
    /// artifacts on visible objects if the camera is moving at sub-pixel distances.
    pub fn pixel_snapped(self) -> WorldPoint {
        // NOTE: Because OpenGL pixels are drawn from the bottom left we need to floor here
        //       to correctly transform world coordinates to pixels.
        WorldPoint {
            x: f32::floor(PIXELS_PER_UNIT * self.x) / PIXELS_PER_UNIT,
            y: f32::floor(PIXELS_PER_UNIT * self.y) / PIXELS_PER_UNIT,
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
/// TODO(JaSc): Copy example from old project
pub struct Camera {
    pub world_rect: Rect,
    pub zoom_level: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Camera {
    pub fn new(screen_width: i32, screen_height: i32, z_near: f32, z_far: f32) -> Camera {
        Camera::with_position(Point::zero(), screen_width, screen_height, z_near, z_far)
    }

    pub fn with_position(
        pos: WorldPoint,
        screen_width: i32,
        screen_height: i32,
        z_near: f32,
        z_far: f32,
    ) -> Camera {
        Camera {
            world_rect: Rect::new(
                pos.x,
                pos.y,
                screen_width as f32 / PIXELS_PER_UNIT,
                screen_height as f32 / PIXELS_PER_UNIT,
            ),
            zoom_level: 1.0,
            z_near,
            z_far,
        }
    }

    pub fn dim_zoomed(&self) -> Vec2 {
        self.world_rect.dim / self.zoom_level
    }

    /// Converts normalized screen coordinates (`[0, 1[x[0, 1[` - bottom-left to top-right)
    /// to world coordinates.
    pub fn screen_to_world(&self, point: ScreenPoint) -> WorldPoint {
        (point - 0.5) * self.dim_zoomed() + self.world_rect.pos.pixel_snapped()
    }

    /// Converts world coordinates to normalized screen coordinates
    /// (`[0, 1[x[0, 1[` - bottom-left to top-right)
    pub fn world_to_screen(&self, point: WorldPoint) -> ScreenPoint {
        (point - self.world_rect.pos.pixel_snapped()) * self.dim_zoomed() + 0.5
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

    /// Pans the camera using cursor movement distance which is given in normalized screen
    /// coordinates (`[0, 1[x[0, 1[` - bottom-left to top-right)
    pub fn pan(&mut self, screen_move_distance: ScreenPoint) {
        self.world_rect.pos -= screen_move_distance * self.dim_zoomed();
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&self) -> Mat4 {
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

    /// Returns the [`Bounds`] of the camera view in world-space
    pub fn bounds_worldspace(&self) -> Bounds {
        let world_rect_zoomed = Rect::from_point_dimension(self.world_rect.pos, self.dim_zoomed());
        world_rect_zoomed.bounds_centered()
    }
}
