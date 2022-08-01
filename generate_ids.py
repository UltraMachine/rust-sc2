from json import load
from pathlib import Path
from sys import argv

HEAD = """\
#![allow(deprecated)]

#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};
"""
DERIVES = """\
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq, Hash)]\
"""
ENUM_NAMES = ("UnitTypeId", "AbilityId", "UpgradeId", "BuffId", "EffectId")
FILE_NAMES = ("unit_typeid", "ability_id", "upgrade_id", "buff_id", "effect_id")

MIMICS = {
	"UnitTypeId": {
		"Lurker": "LurkerMP",
		"LurkerBurrowed": "LurkerMPBurrowed",
		"LurkerDen": "LurkerDenMP",
		"LurkerEgg": "LurkerMPEgg",
	},
	"UpgradeId": {
		"TerranVehicleArmorsLevel1": "TerranVehicleAndShipArmorsLevel1",
		"TerranVehicleArmorsLevel2": "TerranVehicleAndShipArmorsLevel2",
		"TerranVehicleArmorsLevel3": "TerranVehicleAndShipArmorsLevel3",
		"TerranShipArmorsLevel1": "TerranVehicleAndShipArmorsLevel1",
		"TerranShipArmorsLevel2": "TerranVehicleAndShipArmorsLevel2",
		"TerranShipArmorsLevel3": "TerranVehicleAndShipArmorsLevel3",
		"MarineStimpack": "Stimpack",
		"CombatShield": "ShieldWall",
		"JackhammerConcussionGrenades": "PunisherGrenades",
		"InfernalPreIgniters": "HighCapacityBarrels",
		"HellionCampaignInfernalPreIgniter": "HighCapacityBarrels",
		"TransformationServos": "SmartServos",
		"CycloneRapidFireLaunchers": "CycloneLockOnDamageUpgrade",
		"MagFieldLaunchers": "CycloneLockOnDamageUpgrade",
		"PermanentCloakGhost": "PersonalCloaking",
		"YamatoCannon": "BattlecruiserEnableSpecializations",
	},
}


def mimic(id, enum):
	try:
		id = MIMICS[enum][id]
	except KeyError:
		return ""
	return f'\t#[deprecated(note = "Use `{enum}::{id}` instead.")]\n'


def parse_simple(d, data):
	units = {}
	for v in data[d]:
		key = v["name"]

		if not key:
			continue

		key_to_insert = key.replace(" ", "").replace("_", "").replace("@", "")
		if key_to_insert[0].isdigit():
			key_to_insert = "_" + key_to_insert
		if key_to_insert in units:
			index = 2
			tmp = f"{key_to_insert}{index}"
			while tmp in units:
				index += 1
				tmp = f"{key_to_insert}{index}"
			key_to_insert = tmp
		key_to_insert = key_to_insert[0].upper() + key_to_insert[1:]
		units[key_to_insert] = v["id"]

	return units


def parse_data(data, version=None):
	units = parse_simple("Units", data)
	upgrades = parse_simple("Upgrades", data)
	effects = parse_simple("Effects", data)
	buffs = parse_simple("Buffs", data)

	abilities = {}
	for v in data["Abilities"]:
		key = v["buttonname"]
		remapid = v.get("remapid")

		if (not key) and (remapid is None):
			assert v["buttonname"] == ""
			continue

		if not key:
			if v["friendlyname"] != "":
				key = v["friendlyname"]
			else:
				exit(f"Not mapped: {v !r}")

		key = key.replace(" ", "").replace("_", "").replace("@", "")
		if "name" in v:
			key = f'{v["name"].replace(" ", "").replace("_", "").replace("@", "")}{key}'

		if "friendlyname" in v:
			key = v["friendlyname"].replace(" ", "").replace("_", "").replace("@", "")

		if key[0].isdigit():
			key = "_" + key

		key = key[0].upper() + key[1:]
		key = key.replace("ResearchResearch", "Research")

		if key in abilities and v["index"] == 0:
			# print(f"{key} has value 0 and id {v['id']}, overwriting {key}: {abilities[key]}")
			# Commented out to try to fix: 3670 is not a valid AbilityId
			abilities[key] = v["id"]
			pass
		else:
			abilities[key] = v["id"]

	# fixes for wrong ids
	# if version == "4.10":
	#   upgrades["EnhancedShockwaves"] = 296
	#   abilities["GhostAcademyResearchEnhancedShockwaves"] = 822
	# elif version is None:
	if version is None:
		abilities["TerranBuildRefinery"] = 320
	elif version == "linux505":
		units["AssimilatorRich"] = 1980
		units["ExtractorRich"] = 1981
		units["AccelerationZoneSmall"] = 1985
		units["AccelerationZoneMedium"] = 1986
		units["AccelerationZoneLarge"] = 1987

		upgrades["TempestGroundAttackUpgrade"] = 296
		upgrades["EnhancedShockwaves"] = 297

		abilities["FleetBeaconResearchVoidRaySpeedUpgrade"] = 48
		abilities["FleetBeaconResearchTempestResearchGroundAttackUpgrade"] = 49
		abilities["FleetBeaconResearchTempestResearchGroundAttackUpgrade"] = 49
		abilities["GhostAcademyResearchEnhancedShockwaves"] = 822
		abilities["LurkerDenResearchLurkerRange"] = 3710
		abilities["BatteryOverchargeBatteryOvercharge"] = 3815
		abilities["AmorphousArmorcloudAmorphousArmorcloud"] = 3817

		buffs["AccelerationZoneTemporalField"] = 290
		buffs["AmorphousArmorcloud"] = 296
		buffs["BatteryOvercharge"] = 298

	return (
		units,
		abilities,
		upgrades,
		buffs,
		effects,
	)


def gen_enum(enum, name):
	return (
		f"{DERIVES}\npub enum {name} {{\n"
		+ "".join(
			mimic(k, name) + f"\t{k} = {v},\n"
			for k, v in sorted(enum.items(), key=lambda x: x[1])
		)
		+ "}\n"
	)


def generate():
	mod = [
		[
			"//! Auto generated with `generate_ids.py` script from `stableid.json`",
			"//! ids of units, ablities, upgrades, buffs and effects.",
			"#![allow(missing_docs)]",
		],
		[],
		[],
		["mod impls;"],
	]
	enums_latest = parse_data(
		load((Path.home() / "Documents" / "StarCraft II" / "stableid.json").open())
	)
	enums_linux = enums_latest
	# parse_data(
	# 	load(
	# 		(Path.home() / "Documents" / "StarCraft II" / "stableid_4.10.json").open()
	# 	),
	# 	version="linux505",
	# )

	for name, file, enum, enum_linux in zip(
		ENUM_NAMES, FILE_NAMES, enums_latest, enums_linux
	):
		if enum == enum_linux:
			generated = f"{HEAD}\n{gen_enum(enum, name)}"
		else:
			generated = (
				f'{HEAD}\n#[cfg(target_os = "windows")]\n'
				+ gen_enum(enum, name)
				+ '\n#[cfg(target_os = "linux")]\n'
				+ gen_enum(enum_linux, name)
			)

		(Path.cwd() / "src" / "ids" / f"{file}.rs").write_text(generated)
		mod[1].append(f"mod {file};")
		mod[2].append(f"pub use {file}::{name};")
	(Path.cwd() / "src" / "ids" / "mod.rs").write_text(
		"\n\n".join("\n".join(part) for part in mod) + "\n"
	)


if __name__ == "__main__":
	generate()
