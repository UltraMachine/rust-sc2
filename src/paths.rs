use dirs::home_dir;
use regex::Regex;
use std::{env::var_os, fs::read_to_string, path::Path};

pub fn get_path_to_sc2() -> String {
	match var_os("SC2PATH") {
		Some(path) => path.to_str().unwrap().to_string(),
		None => {
			if cfg!(target_os = "windows") {
				let file = read_to_string(format!(
					"{}/Documents/StarCraft II/ExecuteInfo.txt",
					home_dir().unwrap().to_str().unwrap(),
				))
				.expect("Can't read ExecuteInfo.txt");
				let re = Regex::new(r"= (.*)\\Versions").unwrap().captures(&file).unwrap();

				let path = Path::new(&re[1]);
				if path.exists() {
					return path.to_str().unwrap().replace("\\", "/");
				}
			}

			if cfg!(target_os = "windows") {
				"C:/Program Files (x86)/StarCraft II"
			} else if cfg!(target_os = "linux") {
				"~/StarCraftII"
			} else {
				panic!("Unsupported OS")
			}
			.to_string()
		}
	}
}

pub fn get_latest_base_version(sc2_path: &str) -> u32 {
	Path::new(&format!("{}/Versions", sc2_path))
		.read_dir()
		.expect("Can't read `Versions` folder")
		.filter_map(|dir| {
			let dir = dir.unwrap();
			if match dir.file_type() {
				Ok(ftype) => ftype.is_dir(),
				Err(_) => false,
			} {
				match dir.file_name().to_str() {
					Some(name) if name.starts_with("Base") => Some(name[4..].parse::<u32>().unwrap()),
					_ => None,
				}
			} else {
				None
			}
		})
		.max()
		.unwrap()
}

pub fn get_base_version(version: &str) -> u32 {
	match version {
		"4.11.4" => 78285,
		"4.11.3" => 77661,
		"4.11.2" => 77535,
		"4.11.1" => 77474,
		"4.11" | "4.11.0" => 77379,
		"4.10.4" => 76811,
		"4.10.3" => 76114,
		"4.10.2" => 76052,
		"4.10.1" => 75800,
		"4.10" | "4.10.0" => 75689,
		"4.9.3" => 75025,
		"4.9.2" => 74741,
		"4.9.1" => 74456,
		"4.9" | "4.9.0" => 74071,
		"4.8.6" => 73620,
		"4.8.5" => 73559,
		"4.8.4" => 73286,
		"4.8.3" => 72282,
		"4.8.2" => 71663,
		"4.8.1" => 71523,
		"4.8" | "4.8.0" => 71061,
		v => panic!("Can't find base version for `{:?}`", v),
	}
}
