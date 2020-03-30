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
	pub fn distance(self, other: Self) -> f32 {
		let dx = self.x - other.x;
		let dy = self.y - other.y;
		(dx * dx + dy * dy).sqrt()
	}
	pub fn distance_squared(self, other: Self) -> f32 {
		let dx = self.x - other.x;
		let dy = self.y - other.y;
		dx * dx + dy * dy
	}
	pub fn towards(self, other: Self, offset: f32) -> Self {
		self * (offset / self.distance(other))
	}
}
impl PartialEq for Point2 {
	fn eq(&self, other: &Self) -> bool {
		// (self.x as u32) == (other.x as u32) && (self.y as u32) == (other.y as u32)
		self.x == other.x && self.y == other.y
	}
}
impl Eq for Point2 {}
impl Hash for Point2 {
	fn hash<H: Hasher>(&self, state: &mut H) {
		(self.x as u32).hash(state);
		(self.y as u32).hash(state);
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
impl Div for Point2 {
	type Output = Self;

	fn div(self, other: Self) -> Self {
		Self {
			x: self.x / other.x,
			y: self.y / other.y,
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
impl Sub for Point2 {
	type Output = Self;

	fn sub(self, other: Self) -> Self {
		Self {
			x: self.x - other.x,
			y: self.y - other.y,
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
impl Div<f32> for Point2 {
	type Output = Self;

	fn div(self, other: f32) -> Self {
		Self {
			x: self.x / other,
			y: self.y / other,
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
impl Sub<f32> for Point2 {
	type Output = Self;

	fn sub(self, other: f32) -> Self {
		Self {
			x: self.x - other,
			y: self.y - other,
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
