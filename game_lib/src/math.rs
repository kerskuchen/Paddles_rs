pub use cgmath;
pub use cgmath::ortho;
pub use cgmath::prelude::*;

pub const EPSILON: f32 = 0.000_001;
pub const COLLISION_SAFETY_MARGIN: f32 = 0.1;

use std;
pub use std::f32::consts::PI;

pub type Color = cgmath::Vector4<f32>;
pub type Mat4 = cgmath::Matrix4<f32>;

pub fn is_effectively_zero(x: f32) -> bool {
    f32::abs(x) < EPSILON
}

pub fn is_positive(x: f32) -> bool {
    x > EPSILON
}

// TODO(JaSc): Decide if we want to pass self of &self into methods of small copyable types

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
    /// let rect = Rect::from_xy_width_height(0.0, 0.0, 1.5, 1.5);
    /// assert_eq!(Point::new(1.0, 1.5), point.clamped_in_rect(rect));
    ///
    /// ```
    pub fn clamped_in_rect(self, rect: Rect) -> Point {
        Point::new(
            clamp(self.x, rect.left, rect.right),
            clamp(self.y, rect.top, rect.bottom),
        )
    }

    pub fn intersects_line(self, line: Line, line_thickness: f32) -> bool {
        let distance_to_start = self - line.start;
        let distance_to_line = f32::abs(Vec2::dot(distance_to_start, line.normal()));
        distance_to_line <= line_thickness
    }

    pub fn intersects_circle(self, circle: Circle) -> bool {
        self.squared_distance_to(circle.center) <= circle.radius * circle.radius
    }

    pub fn intersects_rect(self, rect: Rect) -> bool {
        rect.left <= self.x && self.x <= rect.right && rect.top <= self.y && self.y <= rect.bottom
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

impl Default for Vec2 {
    fn default() -> Vec2 {
        Vec2::zero()
    }
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

    // Returns a unit vector constructed from an angle in range [-PI, PI]
    // which represents the angle between the resulting vector and the vector (1,0) in the
    // 2D cartesian coordinate system.
    pub fn from_angle(angle: f32) -> Vec2 {
        // NOTE: The y component is negative as a correction for our y-flipped coordinate system
        Vec2::new(f32::cos(angle), -f32::sin(angle))
    }

    pub fn normalized(self) -> Vec2 {
        self / self.magnitude()
    }

    pub fn perpendicular(self) -> Vec2 {
        // NOTE: The y component is positive as a correction for our y-flipped coordinate system
        Vec2::new(self.y, self.x)
    }

    pub fn magnitude(self) -> f32 {
        f32::sqrt(self.x * self.x + self.y * self.y)
    }

    pub fn slid_on_normal(self, normal: Vec2) -> Vec2 {
        self - Vec2::dot(self, normal) * normal
    }

    pub fn reflected_on_normal(self, normal: Vec2) -> Vec2 {
        self - 2.0 * Vec2::dot(self, normal) * normal
    }

    pub fn distance_to(self, other: Vec2) -> f32 {
        Vec2::distance(self, other)
    }

    pub fn squared_distance_to(self, other: Vec2) -> f32 {
        Vec2::squared_distance(self, other)
    }

    pub fn distance(a: Vec2, b: Vec2) -> f32 {
        f32::sqrt((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y))
    }

    pub fn squared_distance(a: Vec2, b: Vec2) -> f32 {
        (a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)
    }

    pub fn dot(a: Vec2, b: Vec2) -> f32 {
        a.x * b.x + a.y * b.y
    }

    // Returns the z-component of a 3D cross-product of `a` and `b` as if they were 3D-vectors
    pub fn cross(a: Vec2, b: Vec2) -> f32 {
        a.x * b.y - a.y * b.x
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
    fn ortho_origin_center_flipped_y(width: f32, height: f32, near: f32, far: f32) -> Self;
    fn ortho_origin_bottom_left(width: f32, height: f32, near: f32, far: f32) -> Self;
    fn ortho_origin_top_left(width: f32, height: f32, near: f32, far: f32) -> Self;
}

impl Mat4Helper for Mat4 {
    fn ortho_origin_center_flipped_y(width: f32, height: f32, near: f32, far: f32) -> Self {
        cgmath::ortho(
            -0.5 * width,
            0.5 * width,
            0.5 * height,
            -0.5 * height,
            near,
            far,
        )
    }

    fn ortho_origin_bottom_left(width: f32, height: f32, near: f32, far: f32) -> Self {
        cgmath::ortho(0.0, width, 0.0, height, near, far)
    }

    fn ortho_origin_top_left(width: f32, height: f32, near: f32, far: f32) -> Self {
        cgmath::ortho(0.0, width, height, 0.0, near, far)
    }
}

//==================================================================================================
// Geometry
//==================================================================================================
//

// TODO(JaSc): Write some docs, unittests/examples for these
/// Origin -> top-left
#[derive(Default, Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Rect {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Rect {
    // ---------------------------------------------------------------------------------------------
    // Constructors
    //
    pub fn zero() -> Rect {
        Default::default()
    }

    pub fn from_bounds(left: f32, right: f32, bottom: f32, top: f32) -> Rect {
        Rect {
            left,
            right,
            top,
            bottom,
        }
    }

    pub fn from_xy_width_height(x: f32, y: f32, width: f32, height: f32) -> Rect {
        Rect {
            left: x,
            right: x + width,
            top: y,
            bottom: y + height,
        }
    }

    pub fn from_width_height(width: f32, height: f32) -> Rect {
        Rect::from_xy_width_height(0.0, 0.0, width, height)
    }

    pub fn from_point(pos: Point, width: f32, height: f32) -> Rect {
        Rect::from_xy_width_height(pos.x, pos.y, width, height)
    }

    pub fn from_dimension(dim: Vec2) -> Rect {
        Rect::from_xy_width_height(0.0, 0.0, dim.x, dim.y)
    }

    pub fn from_point_dimension(pos: Point, dim: Vec2) -> Rect {
        Rect::from_xy_width_height(pos.x, pos.y, dim.x, dim.y)
    }

    pub fn unit_rect() -> Rect {
        Rect {
            left: 0.0,
            right: 1.0,
            top: 0.0,
            bottom: 1.0,
        }
    }

    pub fn unit_rect_centered() -> Rect {
        Rect::unit_rect().centered()
    }

    // ---------------------------------------------------------------------------------------------
    // Accessors
    //
    pub fn pos(&self) -> Point {
        Point::new(self.left, self.top)
    }

    pub fn center(&self) -> Point {
        self.pos() + 0.5 * self.dim()
    }

    pub fn dim(&self) -> Vec2 {
        Vec2::new(self.right - self.left, self.bottom - self.top)
    }

    pub fn width(&self) -> f32 {
        self.right - self.left
    }

    pub fn height(&self) -> f32 {
        self.bottom - self.top
    }

    // ---------------------------------------------------------------------------------------------
    // Modify geometry
    //
    pub fn translated_by(self, translation: Vec2) -> Rect {
        Rect {
            left: self.left + translation.x,
            right: self.right + translation.x,
            top: self.top + translation.y,
            bottom: self.bottom + translation.y,
        }
    }

    pub fn translated_to_origin(self) -> Rect {
        self.translated_by(-self.pos())
    }

    pub fn translated_to_pos(self, pos: Point) -> Rect {
        self.translated_to_origin().translated_by(pos)
    }

    pub fn centered(self) -> Rect {
        let half_dim = 0.5 * self.dim();
        self.translated_by(-half_dim)
    }

    pub fn centered_in_origin(self) -> Rect {
        self.translated_to_origin().centered()
    }

    pub fn centered_in_position(self, pos: Point) -> Rect {
        self.translated_to_pos(pos).centered()
    }

    pub fn scaled_from_origin(self, scale: Vec2) -> Rect {
        debug_assert!(is_positive(scale.x));
        debug_assert!(is_positive(scale.y));
        Rect {
            left: self.left * scale.x,
            right: self.right * scale.x,
            top: self.top * scale.y,
            bottom: self.bottom * scale.y,
        }
    }

    pub fn scaled_from_center(self, scale: Vec2) -> Rect {
        debug_assert!(is_positive(scale.x));
        debug_assert!(is_positive(scale.y));

        let center = self.center();
        self.centered_in_origin()
            .scaled_from_origin(scale)
            .centered_in_position(center)
    }

    pub fn extended_uniformly_by(self, extension: f32) -> Rect {
        Rect {
            left: self.left - extension,
            right: self.right + extension,
            top: self.top - extension,
            bottom: self.bottom + extension,
        }
    }

    /// Returns a version of the rectangle that is centered in a given rect
    pub fn centered_in_rect(self, target: Rect) -> Rect {
        let offset_centered = target.pos() + 0.5 * (target.dim() - self.dim());
        Rect::from_point_dimension(offset_centered, self.dim())
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

        Rect::from_point(self.pos(), stretched_width, stretched_height)
    }

    // ---------------------------------------------------------------------------------------------
    // Intersection
    //

    pub fn intersects_rect(self, other: Rect) -> bool {
        self.right >= other.left
            && self.left <= other.right
            && self.top <= other.bottom
            && self.bottom >= other.top
    }

    // ---------------------------------------------------------------------------------------------
    // Conversions
    //
    pub fn to_border_lines(&self) -> [Line; 4] {
        [
            // Top horizontal line
            Line {
                start: Point {
                    x: self.left,
                    y: self.top,
                },
                end: Point {
                    x: self.right,
                    y: self.top,
                },
            },
            // Right vertical line
            Line {
                start: Point {
                    x: self.right,
                    y: self.top,
                },
                end: Point {
                    x: self.right,
                    y: self.bottom,
                },
            },
            // Bottom horizontal line
            Line {
                start: Point {
                    x: self.right,
                    y: self.bottom,
                },
                end: Point {
                    x: self.left,
                    y: self.bottom,
                },
            },
            // Left vertical line
            Line {
                start: Point {
                    x: self.left,
                    y: self.bottom,
                },
                end: Point {
                    x: self.left,
                    y: self.top,
                },
            },
        ]
    }
}

//==================================================================================================
// Line
//==================================================================================================
//
#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub start: Point,
    pub end: Point,
}

#[derive(Debug, Clone, Copy)]
pub struct Intersection {
    pub point: Point,
    pub normal: Vec2,
    pub time: f32,
    pub segment_index: usize,
}

impl Line {
    pub fn new(start: Point, end: Point) -> Line {
        Line { start, end }
    }

    pub fn to_intersection_point(&self, t: f32) -> Point {
        self.start + t * (self.end - self.start)
    }

    pub fn length(self) -> f32 {
        Vec2::distance(self.start, self.end)
    }

    pub fn squared_length(self) -> f32 {
        Vec2::squared_distance(self.start, self.end)
    }

    pub fn normal(self) -> Vec2 {
        (self.end - self.start).perpendicular().normalized()
    }

    pub fn raycast_with_rects(&self, rects: &[Rect]) -> Option<Intersection> {
        let mut intersections: Vec<Intersection> = rects
            .iter()
            .map(|rect| self.raycast_with_rect(rect, &[]))
            .filter(|maybe_intersection| maybe_intersection.is_some())
            .map(|intersection| intersection.unwrap())
            .collect();

        if intersections.len() == 0 {
            None
        } else if intersections.len() == 1 {
            Some(intersections[0])
        } else {
            intersections.sort_unstable_by(|a, b| {
                f32::partial_cmp(&a.time, &b.time).unwrap_or(std::cmp::Ordering::Equal)
            });
            Some(intersections[0])
        }
    }

    pub fn raycast_with_rect(
        &self,
        rect: &Rect,
        ignored_segment_indices: &[usize],
    ) -> Option<Intersection> {
        self.raycast_with_segments(
            &rect
                .extended_uniformly_by(COLLISION_SAFETY_MARGIN)
                .to_border_lines(),
            ignored_segment_indices,
        )
    }

    // Checks intersection of a line with multiple lines.
    // NOTE: We treat colinear line segments as non-intersecting
    fn raycast_with_segments(
        &self,
        lines: &[Line],
        ignored_segment_indices: &[usize],
    ) -> Option<Intersection> {
        let mut min_intersection_time = std::f32::MAX;
        let mut result = None;

        for (index, line) in lines.iter().enumerate() {
            if ignored_segment_indices.contains(&index) {
                continue;
            }

            if let Some(intersection) = Line::intersect_lines(*self, *line, index) {
                if intersection.time <= min_intersection_time {
                    min_intersection_time = intersection.time;
                    result = Some(intersection);
                }
            }
        }
        result
    }

    // Checks whether two line segments intersect. If so returns the intersection point `point`
    // and the time of intersection `time_a` with `point = a.start + time_a * (a.end - a.start)`.
    // See https://stackoverflow.com/a/565282 for derivation
    // with p = self.start, r = self_dir, q = line.start, s = line_dir.
    // NOTE: We treat colinear line segments as non-intersecting
    pub fn intersect_lines(a: Line, b: Line, segment_index: usize) -> Option<Intersection> {
        let dir_a = a.end - a.start;
        let dir_b = b.end - b.start;
        let dir_a_x_dir_b = Vec2::cross(dir_a, dir_b);

        if !is_effectively_zero(dir_a_x_dir_b) {
            let diff_start_b_a = b.start - a.start;
            let time_a = Vec2::cross(diff_start_b_a, dir_b) / dir_a_x_dir_b;
            let time_b = Vec2::cross(diff_start_b_a, dir_a) / dir_a_x_dir_b;

            // Check if t in [0, 1] and u in [0, 1]
            if time_a >= 0.0 && time_a <= 1.0 && time_b >= 0.0 && time_b <= 1.0 {
                let intersection = Intersection {
                    point: a.start + time_a * dir_a,
                    normal: dir_b.perpendicular().normalized(),
                    time: time_a,
                    segment_index,
                };
                return Some(intersection);
            }
        }

        return None;
    }

    pub fn intersects_rect(self, rect: Rect) -> bool {
        rect.to_border_lines()
            .iter()
            .any(|&line| Line::intersect_lines(self, line, 0).is_some())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Circle {
    pub center: Point,
    pub radius: f32,
}

impl Circle {
    pub fn new(center: Point, radius: f32) -> Circle {
        Circle { center, radius }
    }

    pub fn intersects_line(self, line: Line, line_thickness: f32) -> bool {
        let distance_to_start = self.center - line.start;
        let distance_to_line = f32::abs(Vec2::dot(distance_to_start, line.normal()));
        distance_to_line <= line_thickness + self.radius
    }

    pub fn intersects_circle(self, other: Circle) -> bool {
        let radius_sum = self.radius + other.radius;
        Vec2::squared_distance(self.center, other.center) <= radius_sum * radius_sum
    }

    pub fn intersects_rect(self, rect: Rect) -> bool {
        let rect_point_that_is_nearest_to_circle = Point::new(
            f32::max(rect.left, f32::min(self.center.x, rect.right)),
            f32::max(rect.bottom, f32::min(self.center.y, rect.top)),
        );
        rect_point_that_is_nearest_to_circle.intersects_circle(self)
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
        // NOTE: Because OpenGL pixels are drawn from the top left we need to floor here
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
/// let pos = Point::new(50.0, -50.0);
/// let dim = Vec2::new(200.0, 100.0);
/// let zoom = 2.0;
///
///
/// let cam_origin = Point::new(12.0, 34.0);
/// let mut cam = Camera::new(cam_origin, dim.x, dim.y, -1.0, 1.0);
///
/// // NOTE: Our panning vector is the negative of our move vector. This is to simulate the
/// //       mouse grabbing and panning of the canvas, like i.e. touch-navigation on mobile devices.
/// let move_vec = pos - cam_origin;
/// let panning_vec = -move_vec;
/// cam.pan(panning_vec);
/// assert_eq!(cam.pos(), pos);
///
/// cam.zoom_to_world_point(pos, zoom);
/// assert_eq!(cam.zoom_level, zoom);
/// assert_eq!(cam.dim_zoomed(), dim / zoom);
///
/// let left =   pos.x - 0.5 * dim.x / zoom;
/// let right =  pos.x + 0.5 * dim.x / zoom;
/// let top =    pos.y - 0.5 * dim.y / zoom;
/// let bottom = pos.y + 0.5 * dim.y / zoom;
///
/// let bounds = cam.frustum();
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
/// let canvas_width = 320.0;
/// let canvas_height = 180.0;
/// let mut cam = Camera::new(Point::zero(), canvas_width, canvas_height, -1.0, 1.0);
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
    frustum: Rect,
    pub zoom_level: f32,
    pub z_near: f32,
    pub z_far: f32,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera {
            frustum: Default::default(),
            zoom_level: 1.0,
            z_near: ::DEFAULT_WORLD_ZNEAR,
            z_far: ::DEFAULT_WORLD_ZFAR,
        }
    }
}

impl Camera {
    pub fn new(
        pos: WorldPoint,
        frustum_width: f32,
        frustum_height: f32,
        z_near: f32,
        z_far: f32,
    ) -> Camera {
        Camera {
            frustum: Rect::from_point(pos, frustum_width, frustum_height).centered(),
            zoom_level: 1.0,
            z_near,
            z_far,
        }
    }

    pub fn pos(&self) -> WorldPoint {
        self.frustum.center()
    }

    pub fn dim_zoomed(&self) -> WorldVec {
        self.frustum.dim() / self.zoom_level
    }

    pub fn frustum(&self) -> Rect {
        self.frustum
            .scaled_from_center(Vec2::ones() / self.zoom_level)
    }

    /// Converts a [`CanvasPoint`] into a [`WorldPoint`]
    pub fn canvas_to_world(&self, point: CanvasPoint) -> WorldPoint {
        (point - 0.5 * self.frustum.dim()) / self.zoom_level + self.pos().pixel_snapped()
    }

    /// Converts a [`WorldPoint`] into a [`CanvasPoint`]
    pub fn world_to_canvas(&self, point: WorldPoint) -> CanvasPoint {
        (point - self.pos().pixel_snapped()) * self.zoom_level + 0.5 * self.frustum.dim()
    }

    /// Zooms the camera to or away from a given world point.
    ///
    /// * `new_zoom_level > 1.0` -> magnify
    /// * `new_zoom_level < 1.0` -> minify
    pub fn zoom_to_world_point(&mut self, world_point: WorldPoint, new_zoom_level: f32) {
        let old_zoom_level = self.zoom_level;
        self.zoom_level = new_zoom_level;
        self.frustum = self.frustum.centered_in_position(
            (self.pos() - world_point) * (old_zoom_level / new_zoom_level) + world_point,
        );
    }

    /// Pans the camera using cursor movement distance on the canvas
    pub fn pan(&mut self, canvas_move_distance: CanvasVec) {
        self.frustum = self
            .frustum
            .translated_by(-canvas_move_distance / self.zoom_level);
    }

    /// Returns a project-view-matrix that can transform vertices into camera-view-space
    pub fn proj_view_matrix(&self) -> Mat4 {
        use cgmath::Vector3;

        let translation = -self.pos().pixel_snapped();
        let view_mat = Mat4::from_nonuniform_scale(self.zoom_level, self.zoom_level, 1.0)
            * Mat4::from_translation(Vector3::new(translation.x, translation.y, 0.0));

        let proj_mat = Mat4::ortho_origin_center_flipped_y(
            self.frustum.dim().x,
            self.frustum.dim().y,
            self.z_near,
            self.z_far,
        );
        proj_mat * view_mat
    }
}

//==================================================================================================
// Unit tests
//==================================================================================================
//
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converting_between_canvas_and_world_coordinates_and_back() {
        let cam = Camera::new(WorldPoint::zero(), 100.0, 100.0, -1.0, 1.0);

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
