use crate::{FromProto, IntoProto};
use sc2_proto::common::{Point, Point2D};
use std::{
	hash::{Hash, Hasher},
	iter::Sum,
	ops::{Add, Div, Mul, Sub},
};

#[derive(Debug, Default, Copy, Clone)]
pub struct Size {
	pub x: usize,
	pub y: usize,
}
impl Size {
	pub fn new(x: usize, y: usize) -> Self {
		Self { x, y }
	}
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Rect {
	pub x0: usize,
	pub y0: usize,
	pub x1: usize,
	pub y1: usize,
}
impl Rect {
	pub fn new(x0: usize, y0: usize, x1: usize, y1: usize) -> Self {
		Self { x0, y0, x1, y1 }
	}
}

#[derive(Debug, Default, Copy, Clone)]
pub struct Point2 {
	pub x: f32,
	pub y: f32,
}
impl Point2 {
	pub fn new(x: f32, y: f32) -> Self {
		Self { x, y }
	}
	pub fn distance<P: Into<Point2>>(self, other: P) -> f32 {
		let other = other.into();
		let dx = self.x - other.x;
		let dy = self.y - other.y;
		(dx * dx + dy * dy).sqrt()
	}
	pub fn distance_squared<P: Into<Point2>>(self, other: P) -> f32 {
		let other = other.into();
		let dx = self.x - other.x;
		let dy = self.y - other.y;
		dx * dx + dy * dy
	}
	pub fn towards(self, other: Self, offset: f32) -> Self {
		self + (other - self) / self.distance(other) * offset
	}
	pub fn offset(self, x: f32, y: f32) -> Self {
		Self {
			x: self.x + x,
			y: self.y + y,
		}
	}
	pub fn round(self) -> Self {
		Self {
			x: (self.x + 0.5) as u32 as f32,
			y: (self.y + 0.5) as u32 as f32,
		}
	}
	pub fn neighbors4(self) -> [Self; 4] {
		[
			self.offset(1.0, 0.0),
			self.offset(-1.0, 0.0),
			self.offset(0.0, 1.0),
			self.offset(0.0, -1.0),
		]
	}
	pub fn neighbors4diagonal(self) -> [Self; 4] {
		[
			self.offset(1.0, 1.0),
			self.offset(-1.0, -1.0),
			self.offset(1.0, -1.0),
			self.offset(-1.0, 1.0),
		]
	}
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
	pub fn as_tuple(self) -> (f32, f32) {
		(self.x, self.y)
	}
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
		// ((self.x + 0.5) as u32) == ((other.x + 0.5) as u32) && ((self.y + 0.5) as u32) == ((other.y + 0.5) as u32)
		(self.x - other.x).abs() < std::f32::EPSILON && (self.y - other.y).abs() < std::f32::EPSILON
	}
}
impl Eq for Point2 {}
impl Hash for Point2 {
	fn hash<H: Hasher>(&self, state: &mut H) {
		((self.x + 0.5) as u32).hash(state);
		((self.y + 0.5) as u32).hash(state);
	}
}
impl From<Point2> for (usize, usize) {
	#[inline]
	fn from(p: Point2) -> Self {
		((p.x + 0.5) as usize, (p.y + 0.5) as usize)
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
impl Sum for Point2 {
	fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
		iter.fold(Default::default(), Add::add)
	}
}
impl FromProto<Point2D> for Point2 {
	fn from_proto(p: Point2D) -> Self {
		Self {
			x: p.get_x(),
			y: p.get_y(),
		}
	}
}
impl FromProto<Point> for Point2 {
	fn from_proto(p: Point) -> Self {
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

#[derive(Debug, Default, Copy, Clone)]
pub struct Point3 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}
impl Point3 {
	pub fn new(x: f32, y: f32, z: f32) -> Self {
		Self { x, y, z }
	}
	pub fn offset(self, x: f32, y: f32, z: f32) -> Self {
		Self {
			x: self.x + x,
			y: self.y + y,
			z: self.z + z,
		}
	}
	pub fn round(self) -> Self {
		Self {
			x: (self.x + 0.5) as u32 as f32,
			y: (self.y + 0.5) as u32 as f32,
			z: (self.z + 0.5) as u32 as f32,
		}
	}
	pub fn as_tuple(self) -> (f32, f32, f32) {
		(self.x, self.y, self.z)
	}
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
impl FromProto<Point> for Point3 {
	fn from_proto(p: Point) -> Self {
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
