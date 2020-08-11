//! Data structures, used to store map data.
#![allow(missing_docs)]

use crate::{geometry::Point2, FromProto};
use ndarray::Array2;
use num_traits::FromPrimitive;
use sc2_proto::common::ImageData;
use std::ops::{Index, IndexMut};

/// 2-Dimensional Array of pixels, where each pixel is `Set` or is `Empty`.
pub type PixelMap = Array2<Pixel>;
/// 2-Dimensional Array of bytes.
pub type ByteMap = Array2<u8>;
/// 2-Dimensional Array that represents visibility.
pub type VisibilityMap = Array2<Visibility>;

impl<T> Index<Point2> for Array2<T> {
	type Output = T;

	#[inline]
	fn index(&self, pos: Point2) -> &Self::Output {
		&self[<(usize, usize)>::from(pos)]
	}
}
impl<T> IndexMut<Point2> for Array2<T> {
	#[inline]
	fn index_mut(&mut self, pos: Point2) -> &mut Self::Output {
		&mut self[<(usize, usize)>::from(pos)]
	}
}

fn to_binary(n: u8) -> Vec<Pixel> {
	match n {
		0 => vec![Pixel::Set; 8],
		255 => vec![Pixel::Empty; 8],
		_ => (0..8)
			.rev()
			.map(|x| Pixel::from_u8((n >> x) & 1).unwrap())
			.collect(),
	}
}

impl FromProto<&ImageData> for PixelMap {
	fn from_proto(grid: &ImageData) -> Self {
		let size = grid.get_size();
		Array2::from_shape_vec(
			(size.get_y() as usize, size.get_x() as usize),
			grid.get_data().iter().flat_map(|n| to_binary(*n)).collect(),
		)
		.expect("Can't create PixelMap")
		.reversed_axes()
	}
}
impl FromProto<&ImageData> for ByteMap {
	fn from_proto(grid: &ImageData) -> Self {
		let size = grid.get_size();
		Array2::from_shape_vec(
			(size.get_y() as usize, size.get_x() as usize),
			grid.get_data().iter().copied().collect(),
		)
		.expect("Can't create ByteMap")
		.reversed_axes()
	}
}
impl FromProto<&ImageData> for VisibilityMap {
	fn from_proto(grid: &ImageData) -> Self {
		let size = grid.get_size();
		Array2::from_shape_vec(
			(size.get_y() as usize, size.get_x() as usize),
			grid.get_data()
				.iter()
				.map(|n| {
					Visibility::from_u8(*n)
						.unwrap_or_else(|| panic!("enum Visibility has no variant with value: {}", n))
				})
				.collect(),
		)
		.expect("Can't create VisibilityMap")
		.reversed_axes()
	}
}

/// Base for the most 2d maps.
#[variant_checkers]
#[derive(FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq)]
pub enum Pixel {
	/// When pixel is set, this tile is obstacle (e.g. not pathable | not placeable)
	/// or has something on it (e.g. has creep).
	Set,
	/// When pixel is empty, this tile is free (e.g. pathable | placeable | no creep).
	Empty,
}
impl Default for Pixel {
	fn default() -> Self {
		Pixel::Empty
	}
}
impl std::fmt::Debug for Pixel {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Pixel::Empty => 0.fmt(f),
			Pixel::Set => 1.fmt(f),
		}
	}
}

/// Base for visibility maps.
#[variant_checkers]
#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq)]
pub enum Visibility {
	/// Position is hidden (i.e. weren't explored before)
	Hidden,
	/// Position is in fog of war (i.e. was explored before, but not visible now)
	Fogged,
	/// Position is visible now.
	Visible,
	/// Position is fully hidden (i.e. terrain isn't visible, only darkness; only in campain and custom maps).
	FullHidden,
}
impl Visibility {
	pub fn is_explored(self) -> bool {
		!matches!(self, Visibility::Hidden)
	}
}
impl Default for Visibility {
	fn default() -> Self {
		Visibility::Hidden
	}
}
