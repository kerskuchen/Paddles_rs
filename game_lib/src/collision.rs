use math::*;
use std;

pub type SegmentIndex = usize;
pub type ShapeIndex = usize;
pub const COLLISION_SAFETY_MARGIN: f32 = 0.1;

#[derive(Debug, Clone)]
pub enum CollisionShape {
    Sphere {
        index: ShapeIndex,
        name: &'static str,
        shape: Circle,
    },
    Rect {
        index: ShapeIndex,
        name: &'static str,
        shape: Rect,
    },
    Line {
        index: ShapeIndex,
        name: &'static str,
        shape: Line,
    },
}

#[derive(Debug)]
pub struct CollisionMesh {
    pub name: String,
    pub shapes: Vec<CollisionShape>,
}

#[derive(Debug, Clone)]
pub struct CollisionInfo {
    pub shape: CollisionShape,
    pub segment: CollisionShape,
    pub intersection: Intersection,
}

impl CollisionMesh {
    pub fn new(name: &str) -> CollisionMesh {
        CollisionMesh {
            name: name.to_owned(),
            shapes: Vec::new(),
        }
    }

    pub fn add_rect(&mut self, name: &'static str, rect: Rect) {
        let next_shape_index = self.shapes.len();
        let shape = CollisionShape::Rect {
            index: next_shape_index,
            name,
            shape: rect,
        };
        self.shapes.push(shape);
    }

    pub fn sweepcast_sphere(&self, ray: Line, sphere_radius: f32) -> Option<(CollisionInfo)> {
        let mut collisions = Vec::new();

        for shape in &self.shapes {
            match shape {
                rect @ CollisionShape::Rect { .. } => {
                    let sum_shape = RectSphereSum::new(rect, sphere_radius);
                    if let Some(collision) = raycast_rect_sphere_sum(ray, sum_shape) {
                        collisions.push(collision);
                    }
                }
                _ => panic!("We don't currently support other shape types"),
            }
        }

        // NOTE: We sort intersections in reverse order so we can just pop off the last element
        //       in the vector
        collisions.sort_unstable_by(|a, b| {
            f32::partial_cmp(&b.intersection.time, &a.intersection.time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        collisions.pop()
    }
}

//==================================================================================================
// Minkowski Sums
//==================================================================================================
//
pub struct RectSphereSum {
    pub original_rect: CollisionShape,
    pub shapes: Vec<CollisionShape>,
}

impl RectSphereSum {
    pub fn new(original_rect: &CollisionShape, sphere_radius: f32) -> RectSphereSum {
        if let CollisionShape::Rect { shape, .. } = original_rect {
            let rect = shape;
            let new_left = rect.left - sphere_radius;
            let new_right = rect.right + sphere_radius;
            let new_top = rect.top - sphere_radius;
            let new_bottom = rect.bottom + sphere_radius;

            let shapes = vec![
                CollisionShape::Sphere {
                    index: 0,
                    name: "sphere_top_left",
                    shape: Circle::new(Point::new(rect.left, rect.top), sphere_radius),
                },
                CollisionShape::Sphere {
                    index: 1,
                    name: "sphere_top_right",
                    shape: Circle::new(Point::new(rect.right, rect.top), sphere_radius),
                },
                CollisionShape::Sphere {
                    index: 2,
                    name: "sphere_bottom_left",
                    shape: Circle::new(Point::new(rect.left, rect.bottom), sphere_radius),
                },
                CollisionShape::Sphere {
                    index: 3,
                    name: "sphere_bottom_right",
                    shape: Circle::new(Point::new(rect.right, rect.bottom), sphere_radius),
                },
                // TODO(JaSc): Check if these need to be counter-clockwise
                CollisionShape::Line {
                    index: 4,
                    name: "line_left",
                    shape: Line::new(
                        Point::new(new_left, rect.top),
                        Point::new(new_left, rect.bottom),
                    ),
                },
                CollisionShape::Line {
                    index: 5,
                    name: "line_right",
                    shape: Line::new(
                        Point::new(new_right, rect.top),
                        Point::new(new_right, rect.bottom),
                    ),
                },
                CollisionShape::Line {
                    index: 6,
                    name: "line_top",
                    shape: Line::new(
                        Point::new(rect.left, new_top),
                        Point::new(rect.right, new_top),
                    ),
                },
                CollisionShape::Line {
                    index: 7,
                    name: "line_bottom",
                    shape: Line::new(
                        Point::new(rect.left, new_bottom),
                        Point::new(rect.right, new_bottom),
                    ),
                },
            ];

            RectSphereSum {
                original_rect: original_rect.clone(),
                shapes,
            }
        } else {
            panic!("A RectSphereSum needs a rectangle shape to be passed");
        }
    }

    pub fn to_lines(&self) -> Vec<Line> {
        let mut lines = Vec::new();
        for shape in &self.shapes {
            match shape {
                CollisionShape::Sphere { shape, .. } => lines.append(&mut shape.to_lines(32)),
                CollisionShape::Line { shape, .. } => lines.push(shape.clone()),
                _ => panic!("A RectSphereSum does only consist of spheres and lines"),
            }
        }
        lines
    }
}

//==================================================================================================
// Raycasts and intersections
//==================================================================================================
//

// ---------------------------------------------------------------------------------------------
// Raycasting minkowski sum shapes
//

fn compare_collisions(a: &CollisionInfo, b: &CollisionInfo) -> std::cmp::Ordering {
    compare_floats(a.intersection.time, b.intersection.time)
}

// TODO(JaSc): Allow a way of identifying which part of the original shape we hit
pub fn raycast_rect_sphere_sum(ray: Line, sum: RectSphereSum) -> Option<CollisionInfo> {
    let collision = sum
        .shapes
        .iter()
        .map(|shape| raycast_shape(ray, shape))
        .filter(|maybe_collision| maybe_collision.is_some())
        .map(|collision| collision.unwrap())
        .min_by(|a, b| compare_collisions(a, b));

    collision.map(|collision| CollisionInfo {
        shape: sum.original_rect,
        segment: collision.shape,
        intersection: collision.intersection,
    })
}

// ---------------------------------------------------------------------------------------------
// Raycasting elementary shapes
//

pub fn raycast_shape(ray: Line, shape: &CollisionShape) -> Option<CollisionInfo> {
    let maybe_intersection = match shape {
        CollisionShape::Sphere { shape, .. } => raycast_sphere(ray, *shape),
        CollisionShape::Rect { shape, .. } => raycast_rect(ray, *shape),
        CollisionShape::Line { shape, .. } => raycast_line(ray, *shape),
    };
    maybe_intersection.map(|intersection| CollisionInfo {
        shape: shape.clone(),
        segment: shape.clone(), // TODO(JaSc): How do we get the segments??
        intersection,
    })
}

pub fn raycast_line(ray: Line, line: Line) -> Option<Intersection> {
    intersection_line_line(ray, line)
}

pub fn raycast_sphere(ray: Line, sphere: Circle) -> Option<Intersection> {
    // NOTE: Raycasts from within a sphere are not allowed
    debug_assert!(!ray.start.intersects_sphere(sphere));

    let (intersection_near, _intersection_far) = intersections_line_circle(ray, sphere);
    intersection_near
}

pub fn raycast_rect(ray: Line, rect: Rect) -> Option<Intersection> {
    // TODO(JaSc): We need to solve first how we are going to return which segment of the rect
    //             was hit.
    unimplemented!()
    // NOTE: Raycasts from within a rectangle are not allowed
    //debug_assert!(!ray.start.intersects_rect(rect));
    //pick_closest_intersection(&intersections_line_rect(ray, rect))
}
