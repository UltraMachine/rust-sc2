#[macro_use]
extern crate sc2_macro;
#[macro_use]
extern crate num_derive;

use num_traits::FromPrimitive;

#[derive(Debug, PartialEq, FromPrimitive, FromStr)]
#[enum_from_str(use_primitives)]
enum MyEnum {
	Variant0,
	Variant1 = -1001,
	Variant2,
	Variant3 = 2002,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn check_enum_err() {
		assert_eq!("Variant4".parse::<MyEnum>(), Err(ParseEnumError));
		assert!("Variant4".parse::<MyEnum>().is_err());
		// assert_eq!("MyEnum::Variant4".parse::<MyEnum>(), Err(ParseEnumError));
		assert_eq!("4".parse::<MyEnum>(), Err(ParseEnumError));
	}
	#[test]
	fn check_enum_ok() {
		assert_eq!("Variant1".parse::<MyEnum>(), Ok(MyEnum::Variant1));
		assert_eq!("Variant2".parse::<MyEnum>(), Ok(MyEnum::Variant2));
	}
	/*#[test]
	fn check_enum_ok2() {
		assert_eq!("MyEnum::Variant1".parse::<MyEnum>(), Ok(MyEnum::Variant1));
		assert_eq!("MyEnum::Variant2".parse::<MyEnum>(), Ok(MyEnum::Variant2));
	}*/
	#[test]
	fn check_enum_ok3() {
		assert_eq!("0".parse::<MyEnum>(), Ok(MyEnum::Variant0));
		assert_eq!("-1000".parse::<MyEnum>(), Ok(MyEnum::Variant2));
		assert_eq!("2002".parse::<MyEnum>(), Ok(MyEnum::Variant3));
	}
}
