use crate::FromProto;
use ndarray::Array2;
use sc2_proto::common::ImageData;

pub type PixelMap = Array2<Pixel>;
pub type ByteMap = Array2<u8>;

fn to_binary(n: u8) -> Vec<Pixel> {
	match n {
		0 => vec![Pixel::Empty; 8],
		255 => vec![Pixel::Set; 8],
		_ => {
			let mut n = n;
			let mut bits = Vec::with_capacity(8);
			for _ in 0..8 {
				if n > 0 {
					bits.insert(0, if n % 2 == 0 { Pixel::Empty } else { Pixel::Set });
					n /= 2;
				} else {
					bits.insert(0, Pixel::Empty);
				}
			}
			bits
		}
	}
}

impl FromProto<ImageData> for PixelMap {
	fn from_proto(grid: ImageData) -> Self {
		let size = grid.get_size();
		Array2::from_shape_vec(
			(size.get_x() as usize, size.get_y() as usize),
			grid.get_data().iter().flat_map(|n| to_binary(*n)).collect(),
		)
		.expect("Can't create PixelMap")
	}
}
impl FromProto<ImageData> for ByteMap {
	fn from_proto(grid: ImageData) -> Self {
		let size = grid.get_size();
		Array2::from_shape_vec(
			(size.get_x() as usize, size.get_y() as usize),
			grid.get_data().iter().copied().collect(),
		)
		.expect("Can't create ByteMap")
	}
}

#[derive(FromPrimitive, Copy, Clone)]
pub enum Pixel {
	Empty,
	Set,
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
