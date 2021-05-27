//! Things you liked (hated) at school, now in SC2.
//!
//! Countains various geometric primitives with useful helper methods.

use crate::{distance::Distance, unit::Radius, FromProto, IntoProto};
use sc2_proto::common::{Point, Point2D};
use std::{
	hash::{Hash, Hasher},
	iter::Sum,
	ops::{Add, AddAssign, Div, DivAssign, Mul, MulAssign, Neg, Sub, SubAssign},
};

/// Size of 2D rectangle.
#[allow(missing_docs)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Size {
	pub x: usize,
	pub y: usize,
}
impl Size {
	/// Constructs new `Size` structure with given `x` and `y` size.
	pub fn new(x: usize, y: usize) -> Self {
		Self { x, y }
	}
}

/// Rectangle from (x0, y0) to (x1, y1).
#[allow(missing_docs)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Rect {
	pub x0: usize,
	pub y0: usize,
	pub x1: usize,
	pub y1: usize,
}
impl Rect {
	/// Constructs new rectangle with given coordinates.
	pub fn new(x0: usize, y0: usize, x1: usize, y1: usize) -> Self {
		Self { x0, y0, x1, y1 }
	}
}

/// Point on 2D grid, the most frequently used geometric primitive.
#[allow(missing_docs)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Point2 {
	pub x: f32,
	pub y: f32,
}

#[allow(clippy::len_without_is_empty)]
impl Point2 {
	/// Constructs new 2D Point with given coordinates.
	pub fn new(x: f32, y: f32) -> Self {
		Self { x, y }
	}
	/// Returns new point with offset towards `other` on given distance.
	pub fn towards(self, other: Self, offset: f32) -> Self {
		self + (other - self) / self.distance(other) * offset
	}
	/// Returns new point with offset towards given angle on given distance.
	pub fn towards_angle(self, angle: f32, offset: f32) -> Self {
		self.offset(offset * angle.cos(), offset * angle.sin())
	}
	/// Returns new point with given offset.
	pub fn offset(self, x: f32, y: f32) -> Self {
		Self {
			x: self.x + x,
			y: self.y + y,
		}
	}
	/// Returns points where circles with centers `self` and `other`,
	/// and given radius intersect, or `None` if they aren't intersect.
	pub fn circle_intersection(self, other: Self, radius: f32) -> Option<[Self; 2]> {
		if self == other {
			return None;
		}

		let vec_to_center = (other - self) / 2.0;
		let half_distance = vec_to_center.len();

		if radius < half_distance {
			return None;
		}

		let remaining_distance = (radius * radius - half_distance * half_distance).sqrt();
		let stretch_factor = remaining_distance / half_distance;

		let center = self + vec_to_center;
		let vec_stretched = vec_to_center * stretch_factor;
		Some([
			center + vec_stretched.rotate90(true),
			center + vec_stretched.rotate90(false),
		])
	}

	/// Returns squared length of the vector.
	pub fn len_squared(self) -> f32 {
		self.x.powi(2) + self.y.powi(2)
	}
	/// Returns length of the vector.
	pub fn len(self) -> f32 {
		self.len_squared().sqrt()
	}
	/// Normalizes the vector.
	pub fn normalize(self) -> Self {
		self / self.len()
	}
	/// Rotates the vector on given angle.
	pub fn rotate(self, angle: f32) -> Self {
		let (s, c) = angle.sin_cos();
		let (x, y) = (self.x, self.y);
		Self {
			x: c * x - s * y,
			y: s * x + c * y,
		}
	}
	/// Fast rotation of the vector on 90 degrees.
	pub fn rotate90(self, clockwise: bool) -> Self {
		if clockwise {
			Self::new(self.y, -self.x)
		} else {
			Self::new(-self.y, self.x)
		}
	}
	/// Dot product.
	pub fn dot(self, other: Self) -> f32 {
		self.x * other.x + self.y * other.y
	}

	/// Returns rounded point.
	pub fn round(self) -> Self {
		Self {
			x: (self.x + 0.5) as i32 as f32,
			y: (self.y + 0.5) as i32 as f32,
		}
	}
	/// Returns point rounded to closest lower integer.
	pub fn floor(self) -> Self {
		Self {
			x: self.x as i32 as f32,
			y: self.y as i32 as f32,
		}
	}
	/// Returns point rounded to closest greater integer.
	pub fn ceil(self) -> Self {
		Self {
			x: (self.x + 0.999999) as i32 as f32,
			y: (self.y + 0.999999) as i32 as f32,
		}
	}
	/// Returns point with absolute coordinates.
	pub fn abs(self) -> Self {
		Self {
			x: self.x.abs(),
			y: self.y.abs(),
		}
	}
	/// Returns 4 closest neighbors of point.
	pub fn neighbors4(self) -> [Self; 4] {
		[
			self.offset(1.0, 0.0),
			self.offset(-1.0, 0.0),
			self.offset(0.0, 1.0),
			self.offset(0.0, -1.0),
		]
	}
	/// Returns 4 closest diagonal neighbors of point.
	pub fn neighbors4diagonal(self) -> [Self; 4] {
		[
			self.offset(1.0, 1.0),
			self.offset(-1.0, -1.0),
			self.offset(1.0, -1.0),
			self.offset(-1.0, 1.0),
		]
	}
	/// Returns 8 closest neighbors of point.
	pub fn neighbors8(self) -> [Self; 8] {
		[
			self.offset(1.0, 0.0),
			self.offset(-1.0, 0.0),
			self.offset(0.0, 1.0),
			self.offset(0.0, -1.0),
			self.offset(1.0, 1.0),
			self.offset(-1.0, -1.0),
			self.offset(1.0, -1.0),
			self.offset(-1.0, 1.0),
		]
	}
	/// Returns tuple with point's coordinates.
	pub fn as_tuple(self) -> (f32, f32) {
		(self.x, self.y)
	}
	/// Converts 2D Point to 3D Point using given `z` value.
	pub fn to3(self, z: f32) -> Point3 {
		Point3 {
			x: self.x,
			y: self.y,
			z,
		}
	}
}

impl PartialEq for Point2 {
	fn eq(&self, other: &Self) -> bool {
		// (self.x - other.x).abs() < f32::EPSILON && (self.y - other.y).abs() < f32::EPSILON
		self.x as i32 == other.x as i32 && self.y as i32 == other.y as i32
	}
}
impl Eq for Point2 {}
impl Hash for Point2 {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.x as i32).hash(state);
		(self.y as i32).hash(state);
	}
}

impl From<&Point2> for Point2 {
	#[inline]
	fn from(p: &Point2) -> Self {
		*p
	}
}
impl From<Point2> for (usize, usize) {
	#[inline]
	fn from(p: Point2) -> Self {
		(p.x as usize, p.y as usize)
	}
}
impl From<(usize, usize)> for Point2 {
	#[inline]
	fn from((x, y): (usize, usize)) -> Self {
		Self {
			x: x as f32 + 0.5,
			y: y as f32 + 0.5,
		}
	}
}
impl From<(f32, f32)> for Point2 {
	#[inline]
	fn from((x, y): (f32, f32)) -> Self {
		Self { x, y }
	}
}
impl From<Point2> for (f32, f32) {
	#[inline]
	fn from(p: Point2) -> Self {
		p.as_tuple()
	}
}

impl Add for Point2 {
	type Output = Self;

	fn add(self, other: Self) -> Self {
		Self {
			x: self.x + other.x,
			y: self.y + other.y,
		}
	}
}
impl Sub for Point2 {
	type Output = Self;

	fn sub(self, other: Self) -> Self {
		Self {
			x: self.x - other.x,
			y: self.y - other.y,
		}
	}
}
impl Mul for Point2 {
	type Output = Self;

	fn mul(self, other: Self) -> Self {
		Self {
			x: self.x * other.x,
			y: self.y * other.y,
		}
	}
}
impl Div for Point2 {
	type Output = Self;

	fn div(self, other: Self) -> Self {
		Self {
			x: self.x / other.x,
			y: self.y / other.y,
		}
	}
}
impl AddAssign for Point2 {
	fn add_assign(&mut self, other: Self) {
		self.x += other.x;
		self.y += other.y;
	}
}
impl SubAssign for Point2 {
	fn sub_assign(&mut self, other: Self) {
		self.x -= other.x;
		self.y -= other.y;
	}
}
impl MulAssign for Point2 {
	fn mul_assign(&mut self, other: Self) {
		self.x *= other.x;
		self.y *= other.y;
	}
}
impl DivAssign for Point2 {
	fn div_assign(&mut self, other: Self) {
		self.x /= other.x;
		self.y /= other.y;
	}
}
impl Add<f32> for Point2 {
	type Output = Self;

	fn add(self, other: f32) -> Self {
		Self {
			x: self.x + other,
			y: self.y + other,
		}
	}
}
impl Sub<f32> for Point2 {
	type Output = Self;

	fn sub(self, other: f32) -> Self {
		Self {
			x: self.x - other,
			y: self.y - other,
		}
	}
}
impl Mul<f32> for Point2 {
	type Output = Self;

	fn mul(self, other: f32) -> Self {
		Self {
			x: self.x * other,
			y: self.y * other,
		}
	}
}
impl Div<f32> for Point2 {
	type Output = Self;

	fn div(self, other: f32) -> Self {
		Self {
			x: self.x / other,
			y: self.y / other,
		}
	}
}
impl AddAssign<f32> for Point2 {
	fn add_assign(&mut self, other: f32) {
		self.x += other;
		self.y += other;
	}
}
impl SubAssign<f32> for Point2 {
	fn sub_assign(&mut self, other: f32) {
		self.x -= other;
		self.y -= other;
	}
}
impl MulAssign<f32> for Point2 {
	fn mul_assign(&mut self, other: f32) {
		self.x *= other;
		self.y *= other;
	}
}
impl DivAssign<f32> for Point2 {
	fn div_assign(&mut self, other: f32) {
		self.x /= other;
		self.y /= other;
	}
}
impl Neg for Point2 {
	type Output = Self;

	fn neg(self) -> Self {
		Self {
			x: -self.x,
			y: -self.y,
		}
	}
}
impl Sum for Point2 {
	fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
		iter.fold(Default::default(), Add::add)
	}
}

impl FromProto<&Point2D> for Point2 {
	fn from_proto(p: &Point2D) -> Self {
		Self {
			x: p.get_x(),
			y: p.get_y(),
		}
	}
}
impl FromProto<&Point> for Point2 {
	fn from_proto(p: &Point) -> Self {
		Self {
			x: p.get_x(),
			y: p.get_y(),
		}
	}
}
impl IntoProto<Point2D> for Point2 {
	fn into_proto(self) -> Point2D {
		let mut pos = Point2D::new();
		pos.set_x(self.x);
		pos.set_y(self.y);
		pos
	}
}

/// Point in 3D game world.
#[allow(missing_docs)]
#[derive(Debug, Default, Copy, Clone)]
pub struct Point3 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}
impl Point3 {
	/// Constructs new 3D Point with given coordinates.
	pub fn new(x: f32, y: f32, z: f32) -> Self {
		Self { x, y, z }
	}
	/// Returns new point with given offset.
	pub fn offset(self, x: f32, y: f32, z: f32) -> Self {
		Self {
			x: self.x + x,
			y: self.y + y,
			z: self.z + z,
		}
	}
	/// Returns rounded point.
	pub fn round(self) -> Self {
		Self {
			x: (self.x + 0.5) as i32 as f32,
			y: (self.y + 0.5) as i32 as f32,
			z: (self.z + 0.5) as i32 as f32,
		}
	}
	/// Returns tuple with point's coordinates.
	pub fn as_tuple(self) -> (f32, f32, f32) {
		(self.x, self.y, self.z)
	}
	/// Converts 3D Point to 2D Point.
	pub fn to2(self) -> Point2 {
		Point2 { x: self.x, y: self.y }
	}
}

impl From<Point3> for Point2 {
	#[inline]
	fn from(p3: Point3) -> Self {
		p3.to2()
	}
}

impl From<(f32, f32, f32)> for Point3 {
	#[inline]
	fn from((x, y, z): (f32, f32, f32)) -> Self {
		Self { x, y, z }
	}
}
impl From<Point3> for (f32, f32, f32) {
	#[inline]
	fn from(p3: Point3) -> Self {
		p3.as_tuple()
	}
}

impl Add for Point3 {
	type Output = Self;

	fn add(self, other: Self) -> Self {
		Self {
			x: self.x + other.x,
			y: self.y + other.y,
			z: self.z + other.z,
		}
	}
}
impl Sub for Point3 {
	type Output = Self;

	fn sub(self, other: Self) -> Self {
		Self {
			x: self.x - other.x,
			y: self.y - other.y,
			z: self.z - other.z,
		}
	}
}
impl Mul for Point3 {
	type Output = Self;

	fn mul(self, other: Self) -> Self {
		Self {
			x: self.x * other.x,
			y: self.y * other.y,
			z: self.z * other.z,
		}
	}
}
impl Div for Point3 {
	type Output = Self;

	fn div(self, other: Self) -> Self {
		Self {
			x: self.x / other.x,
			y: self.y / other.y,
			z: self.z / other.z,
		}
	}
}
impl Add<f32> for Point3 {
	type Output = Self;

	fn add(self, other: f32) -> Self {
		Self {
			x: self.x + other,
			y: self.y + other,
			z: self.z + other,
		}
	}
}
impl Sub<f32> for Point3 {
	type Output = Self;

	fn sub(self, other: f32) -> Self {
		Self {
			x: self.x - other,
			y: self.y - other,
			z: self.z - other,
		}
	}
}
impl Mul<f32> for Point3 {
	type Output = Self;

	fn mul(self, other: f32) -> Self {
		Self {
			x: self.x * other,
			y: self.y * other,
			z: self.z * other,
		}
	}
}
impl Div<f32> for Point3 {
	type Output = Self;

	fn div(self, other: f32) -> Self {
		Self {
			x: self.x / other,
			y: self.y / other,
			z: self.z / other,
		}
	}
}
impl Sum for Point3 {
	fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
		iter.fold(Default::default(), Add::add)
	}
}
impl FromProto<&Point> for Point3 {
	fn from_proto(p: &Point) -> Self {
		Self {
			x: p.get_x(),
			y: p.get_y(),
			z: p.get_z(),
		}
	}
}
impl IntoProto<Point> for Point3 {
	fn into_proto(self) -> Point {
		let mut pos = Point::new();
		pos.set_x(self.x);
		pos.set_y(self.y);
		pos.set_z(self.z);
		pos
	}
}

impl Radius for Point2 {}
impl Radius for &Point2 {}
impl Radius for Point3 {}
impl Radius for &Point3 {}
