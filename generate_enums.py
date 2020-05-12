from json import load
from pathlib import Path
from sys import argv

DERIVES = "#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq, Hash)]"
ENUM_NAMES = ("UnitTypeId", "AbilityId", "UpgradeId", "BuffId", "EffectId")
FILE_NAMES = ("unit_typeid", "ability_id", "upgrade_id", "buff_id", "effect_id")


def parse_simple(d, data):
	units = {}
	for v in data[d]:
		key = v["name"]

		if not key:
			continue

		key_to_insert = key.replace(" ", "").replace("_", "")
		if key_to_insert[0].isdigit():
			key_to_insert = "_" + key_to_insert
		if key_to_insert in units:
			index = 2
			tmp = f"{key_to_insert}{index}"
			while tmp in units:
				index += 1
				tmp = f"{key_to_insert}{index}"
			key_to_insert = tmp
		key_to_insert = key_to_insert[0].upper()+key_to_insert[1:]
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

		key = key.replace(" ", "").replace("_", "")
		if "name" in v:
			key = f'{v["name"].replace(" ", "").replace("_", "")}{key}'

		if "friendlyname" in v:
			key = v["friendlyname"].replace(" ", "").replace("_", "")

		if key[0].isdigit():
			key = "_" + key

		key = key[0].upper()+key[1:]
		key = key.replace("ResearchResearch", "Research")

		if key in abilities and v["index"] == 0:
			# print(f"{key} has value 0 and id {v['id']}, overwriting {key}: {abilities[key]}")
			# Commented out to try to fix: 3670 is not a valid AbilityId
			abilities[key] = v["id"]
			pass
		else:
			abilities[key] = v["id"]

	# fixes for wrong ids
	if version == '4.10':
		upgrades['EnhancedShockwaves'] = 296
		abilities['GhostAcademyResearchEnhancedShockwaves'] = 822
	elif version is None:
		abilities['TerranBuildRefinery'] = 320

	return (
		units,
		abilities,
		upgrades,
		buffs,
		effects,
	)


def generate():
	mod = [[], [], ["mod impls;"]]
	enums_latest = parse_data(load((Path.home()/'Documents'/'StarCraft II'/'stableid.json').open()))
	enums_4_10 = parse_data(
		load((Path.home()/'Documents'/'StarCraft II'/'stableid_4.10.json').open()),
		version='4.10'
	)

	for name, file, enum, enum_linux in zip(ENUM_NAMES, FILE_NAMES, enums_latest, enums_4_10):
		if enum == enum_linux:
			generated = f'{DERIVES}\npub enum {name} {{\n'+''.join(
				f'\t{k} = {v},\n'
				for k, v in sorted(enum.items(), key=lambda x: x[1])
			)+'}\n'
		else:
			generated = f'#[cfg(target_os = "windows")]\n{DERIVES}\npub enum {name} {{\n'+''.join(
				f'\t{k} = {v},\n'
				for k, v in sorted(enum.items(), key=lambda x: x[1])
			)+'}\n\n'+f'#[cfg(target_os = "linux")]\n{DERIVES}\npub enum {name} {{\n'+''.join(
				f'\t{k} = {v},\n'
				for k, v in sorted(enum_linux.items(), key=lambda x: x[1])
			)+'}\n'

		(Path.cwd()/'src'/'ids'/f'{file}.rs').write_text(generated)
		mod[0].append(f'mod {file};')
		mod[1].append(f'pub use {file}::{name};')
	(Path.cwd()/'src'/'ids'/'mod.rs').write_text('\n\n'.join('\n'.join(part) for part in mod)+'\n')


if __name__ == '__main__':
	generate()
