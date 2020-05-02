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
			dir.file_type().ok().filter(|ftype| ftype.is_dir()).and(
				dir.file_name()
					.to_str()
					.filter(|name| name.starts_with("Base"))
					.map(|name| name[4..].parse::<u32>().unwrap()),
			)
		})
		.max()
		.unwrap()
}

// Returns (Base version, Data hash)
pub fn get_version_info(version: &str) -> (u32, &str) {
	match version {
		"4.11.4" => (78285, "69493AFAB5C7B45DDB2F3442FD60F0CF"),
		"4.11.3" => (77661, "A15B8E4247434B020086354F39856C51"),
		"4.11.2" => (77535, "FC43E0897FCC93E4632AC57CBC5A2137"),
		"4.11.1" => (77474, "F92D1127A291722120AC816F09B2E583"),
		"4.11" | "4.11.0" => (77379, "70E774E722A58287EF37D487605CD384"),
		"4.10.4" => (76811, "A15B8E4247434B020086354F39856C51"),
		"4.10.3" => (76114, "CDB276D311F707C29BA664B7754A7293"),
		"4.10.2" => (76052, "D0F1A68AA88BA90369A84CD1439AA1C3"),
		"4.10.1" => (75800, "DDFFF9EC4A171459A4F371C6CC189554"),
		"4.10" | "4.10.0" => (75689, "B89B5D6FA7CBF6452E721311BFBC6CB2"),
		"4.9.3" => (75025, "C305368C63621480462F8F516FB64374"),
		"4.9.2" => (74741, "614480EF79264B5BD084E57F912172FF"),
		"4.9.1" => (74456, "218CB2271D4E2FA083470D30B1A05F02"),
		"4.9" | "4.9.0" => (74071, "70C74A2DCA8A0D8E7AE8647CAC68ACCA"),
		"4.8.6" => (73620, "AA18FEAD6573C79EF707DF44ABF1BE61"),
		"4.8.5" => (73559, "B2465E73AED597C74D0844112D582595"),
		"4.8.4" => (73286, "CD040C0675FD986ED37A4CA3C88C8EB5"),
		"4.8.3" => (72282, "0F14399BBD0BA528355FF4A8211F845B"),
		"4.8.2" => (71663, "FE90C92716FC6F8F04B74268EC369FA5"),
		"4.8.1" => (71523, "FCAF3F050B7C0CC7ADCF551B61B9B91E"),
		"4.8" | "4.8.0" => (71061, "760581629FC458A1937A05ED8388725B"),
		v => panic!("Can't find info about version: `{:?}`", v),
	}
}
