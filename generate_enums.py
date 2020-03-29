from json import load
from pathlib import Path
from sys import argv


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

def parse_data(data, version):
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
			print(f"{key} has value 0 and id {v['id']}, overwriting {key}: {abilities[key]}")
			# Commented out to try to fix: 3670 is not a valid AbilityId
			abilities[key] = v["id"]
			pass
		else:
			abilities[key] = v["id"]

	abilities["Smart"] = 1

	if version == '4.10':
		upgrades['EnhancedShockwaves'] = 296
		abilities['GhostAcademyResearchEnhancedShockwaves'] = 822
	elif version == '4.11' or version is None:
		abilities['TerranBuildRefinery'] = 320
		units['CollapsibleRockTowerPushUnitRampRightGreen'] = 1976
		units['MineralField450'] = 1982

	return (
		("UnitTypeId", units, "unit_typeid"),
		("AbilityId", abilities, "ability_id"),
		("UpgradeId", upgrades, "upgrade_id"),
		("BuffId", buffs, "buff_id"),
		("EffectId", effects, "effect_id"),
	)

def generate(version=None):
	mod = [[], []]
	if version is not None:
		stableid_json = f'stableid.json.{version.replace(".", "_")}'
	else:
		stableid_json = f'stableid.json'

	for name, enum, file in parse_data(
			load((Path.home()/'Documents'/'StarCraft II'/stableid_json).open()),
			version,
		):
		generated = f'#[derive(Debug, FromPrimitive, ToPrimitive, Copy, Clone, PartialEq, Eq, Hash)]\npub enum {name} {{\n'+''.join(
			f'\t{k} = {v},\n'
			for k, v in sorted(enum.items(), key=lambda x: x[1])
		)+'}\n'
		(Path.cwd()/'src'/'ids'/f'{file}.rs').write_text(generated)
		mod[0].append(f'mod {file};')
		mod[1].append(f'pub use {file}::{name};')
	(Path.cwd()/'src'/'ids'/'mod.rs').write_text('\n\n'.join('\n'.join(part) for part in mod)+'\n')

if __name__ == '__main__':
	try:
		generate(argv[1])
	except IndexError:
		generate()
