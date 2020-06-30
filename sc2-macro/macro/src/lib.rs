#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use regex::Regex;
use syn::{
	parse_macro_input, Attribute, Data, DeriveInput, Expr, Fields, ItemEnum, ItemFn, ItemStruct, Meta,
	NestedMeta, Stmt,
};

#[proc_macro_attribute]
pub fn bot(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let attr = parse_macro_input!(attr as AttributeArgs);
	let item = parse_macro_input!(item as ItemStruct);

	let name = item.ident;
	let vis = item.vis;
	let attrs = item.attrs;
	let generics = item.generics;
	let fields = match item.fields {
		Fields::Named(named_fields) => {
			let named = named_fields.named;
			quote! {#named}
		}
		Fields::Unnamed(_) => panic!("#[bot] is not allowed for tuple structs"),
		unit @ Fields::Unit => quote! {#unit},
	};

	TokenStream::from(quote! {
		#(#attrs)*
		#vis struct #name#generics {
			_bot: rust_sc2::bot::Bot,
			#fields
		}
		impl std::ops::Deref for #name {
			type Target = rust_sc2::bot::Bot;

			fn deref(&self) -> &Self::Target {
				&self._bot
			}
		}
		impl std::ops::DerefMut for #name {
			fn deref_mut(&mut self) -> &mut Self::Target {
				&mut self._bot
			}
		}
	})
}

#[proc_macro_attribute]
pub fn bot_new(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let attr = parse_macro_input!(attr as AttributeArgs);
	let item = parse_macro_input!(item as ItemFn);

	let vis = item.vis;
	let signature = item.sig;
	let blocks = item.block.stmts.iter().map(|s| match s {
		Stmt::Expr(expr) => match expr {
			Expr::Struct(struct_expr) => {
				let path = &struct_expr.path;
				let rest = match &struct_expr.rest {
					Some(expr) => quote! {#expr},
					None => quote! {},
				};
				let fields = struct_expr.fields.iter();

				quote! {
					#path {
						_bot: Default::default(),
						#(#fields,)*
						..#rest
					}
				}
			}
			n => quote! {#n},
		},
		n => quote! {#n},
	});

	TokenStream::from(quote! {
		#vis #signature {
			#(#blocks)*
		}
	})
}

#[proc_macro_derive(FromStr, attributes(enum_from_str))]
pub fn enum_from_str_derive(input: TokenStream) -> TokenStream {
	let item = parse_macro_input!(input as DeriveInput);
	if let Data::Enum(data) = item.data {
		let name = item.ident;
		let variants = data.variants.iter().map(|v| &v.ident);
		// let variants2 = variants.clone().map(|v| format!("{}::{}", name, v));

		let additional_attributes = |a: &Attribute| {
			if a.path.is_ident("enum_from_str") {
				if let Meta::List(list) = a.parse_meta().unwrap() {
					return list.nested.iter().any(|n| {
						if let NestedMeta::Meta(Meta::Path(path)) = n {
							path.is_ident("use_primitives")
						} else {
							false
						}
					});
				} else {
					unreachable!("No options found in attribute `enum_from_str`")
				}
			}
			false
		};
		let other_cases = if item.attrs.iter().any(additional_attributes) {
			quote! {
				n => {
					if let Ok(num) = n.parse() {
						if let Some(result) = Self::from_i64(num) {
							return Ok(result);
						}
					}
					return Err(sc2_macro::ParseEnumError);
				}
			}
		} else {
			quote! {_ => return Err(sc2_macro::ParseEnumError)}
		};
		TokenStream::from(quote! {
			impl std::str::FromStr for #name {
				type Err = sc2_macro::ParseEnumError;

				fn from_str(s: &str) -> Result<Self, Self::Err> {
					Ok(match s {
						#(
							stringify!(#variants) => Self::#variants,
							// #variants2 => Self::#variants,

						)*
						#other_cases,
					})
				}
			}
		})
	} else {
		panic!("Can only derive FromStr for enums")
	}
}

#[proc_macro_attribute]
pub fn variant_checkers(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let attr = parse_macro_input!(attr as AttributeArgs);
	let item = parse_macro_input!(item as ItemEnum);

	let name = &item.ident;
	let variants = item.variants.iter().map(|v| &v.ident);
	let re = Regex::new(r"[A-Z0-9]{1}[a-z0-9]*").unwrap();
	let snake_variants = variants.clone().map(|v| {
		format_ident!(
			"is_{}",
			re.find_iter(&v.to_string())
				.map(|m| m.as_str().to_ascii_lowercase())
				.collect::<Vec<String>>()
				.join("_")
		)
	});

	TokenStream::from(quote! {
		#item
		impl #name {
			#(
				#[inline]
				pub fn #snake_variants(self) -> bool {
					matches!(self, Self::#variants)
				}
			)*
		}
	})
}
