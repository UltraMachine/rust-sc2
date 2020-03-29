use crate::{
	ids::{AbilityId, BuffId, EffectId, UnitTypeId, UpgradeId},
	player::Race,
	FromProto, TryFromProto,
};
use num_traits::FromPrimitive;
use sc2_proto::{
	data::{
		AbilityData as ProtoAbilityData, AbilityData_Target, Attribute as ProtoAttribute,
		BuffData as ProtoBuffData, EffectData as ProtoEffectData, UnitTypeData as ProtoUnitTypeData,
		UpgradeData as ProtoUpgradeData, Weapon as ProtoWeapon, Weapon_TargetType,
	},
	sc2api::ResponseData,
};
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct GameData {
	pub abilities: HashMap<AbilityId, AbilityData>,
	pub units: HashMap<UnitTypeId, UnitTypeData>,
	pub upgrades: HashMap<UpgradeId, UpgradeData>,
	pub buffs: HashMap<BuffId, BuffData>,
	pub effects: HashMap<EffectId, EffectData>,
}
impl FromProto<ResponseData> for GameData {
	fn from_proto(data: ResponseData) -> Self {
		Self {
			abilities: data
				.get_abilities()
				.iter()
				.filter_map(|a| {
					if let Some(data) = AbilityData::try_from_proto(a.clone()) {
						return Some((data.id, data));
					}
					None
				})
				.collect(),
			units: data
				.get_units()
				.iter()
				.filter_map(|u| {
					if let Some(data) = UnitTypeData::try_from_proto(u.clone()) {
						return Some((data.id, data));
					}
					None
				})
				.collect(),
			upgrades: data
				.get_upgrades()
				.iter()
				.filter_map(|u| {
					if let Some(data) = UpgradeData::try_from_proto(u.clone()) {
						return Some((data.id, data));
					}
					None
				})
				.collect(),
			buffs: data
				.get_buffs()
				.iter()
				.filter_map(|b| {
					if let Some(data) = BuffData::try_from_proto(b.clone()) {
						return Some((data.id, data));
					}
					None
				})
				.collect(),
			effects: data
				.get_effects()
				.iter()
				.filter_map(|e| {
					if let Some(data) = EffectData::try_from_proto(e.clone()) {
						return Some((data.id, data));
					}
					None
				})
				.collect(),
		}
	}
}

#[derive(Clone)]
pub enum AbilityTarget {
	None,
	Point,
	Unit,
	PointOrUnit,
	PointOrNone,
}
impl FromProto<AbilityData_Target> for AbilityTarget {
	fn from_proto(target: AbilityData_Target) -> Self {
		match target {
			AbilityData_Target::None => AbilityTarget::None,
			AbilityData_Target::Point => AbilityTarget::Point,
			AbilityData_Target::Unit => AbilityTarget::Unit,
			AbilityData_Target::PointOrUnit => AbilityTarget::PointOrUnit,
			AbilityData_Target::PointOrNone => AbilityTarget::PointOrNone,
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub enum Attribute {
	Light,
	Armored,
	Biological,
	Mechanical,
	Robotic,
	Psionic,
	Massive,
	Structure,
	Hover,
	Heroic,
	Summoned,
}
impl FromProto<ProtoAttribute> for Attribute {
	fn from_proto(attribute: ProtoAttribute) -> Self {
		match attribute {
			ProtoAttribute::Light => Attribute::Light,
			ProtoAttribute::Armored => Attribute::Armored,
			ProtoAttribute::Biological => Attribute::Biological,
			ProtoAttribute::Mechanical => Attribute::Mechanical,
			ProtoAttribute::Robotic => Attribute::Robotic,
			ProtoAttribute::Psionic => Attribute::Psionic,
			ProtoAttribute::Massive => Attribute::Massive,
			ProtoAttribute::Structure => Attribute::Structure,
			ProtoAttribute::Hover => Attribute::Hover,
			ProtoAttribute::Heroic => Attribute::Heroic,
			ProtoAttribute::Summoned => Attribute::Summoned,
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
pub enum TargetType {
	Ground,
	Air,
	Any,
}
impl FromProto<Weapon_TargetType> for TargetType {
	fn from_proto(target_type: Weapon_TargetType) -> Self {
		match target_type {
			Weapon_TargetType::Ground => TargetType::Ground,
			Weapon_TargetType::Air => TargetType::Air,
			Weapon_TargetType::Any => TargetType::Any,
		}
	}
}

#[derive(Clone)]
pub struct Weapon {
	pub target: TargetType,
	pub damage: f32,
	pub damage_bonus: Vec<(Attribute, f32)>,
	pub attacks: u32,
	pub range: f32,
	pub speed: f32,
}
impl FromProto<ProtoWeapon> for Weapon {
	fn from_proto(weapon: ProtoWeapon) -> Self {
		Self {
			target: TargetType::from_proto(weapon.get_field_type()),
			damage: weapon.get_damage(),
			damage_bonus: weapon
				.get_damage_bonus()
				.iter()
				.map(|db| (Attribute::from_proto(db.get_attribute()), db.get_bonus()))
				.collect(),
			attacks: weapon.get_attacks(),
			range: weapon.get_range(),
			speed: weapon.get_speed(),
		}
	}
}

#[derive(Clone)]
pub struct AbilityData {
	pub id: AbilityId,
	pub link_name: String,
	pub link_index: u32,
	pub button_name: Option<String>,
	pub friendly_name: Option<String>,
	pub hotkey: Option<String>,
	pub remaps_to_ability_id: Option<AbilityId>,
	pub available: bool,
	pub target: AbilityTarget,
	pub allow_minimap: bool,
	pub allow_autocast: bool,
	pub is_building: bool,
	pub footprint_radius: Option<f32>,
	pub is_instant_placement: bool,
	pub cast_range: Option<f32>,
}
impl TryFromProto<ProtoAbilityData> for AbilityData {
	fn try_from_proto(a: ProtoAbilityData) -> Option<Self> {
		Some(Self {
			id: match AbilityId::from_u32(a.get_ability_id()) {
				Some(id) => id,
				None => return None,
			},
			link_name: a.get_link_name().to_string(),
			link_index: a.get_link_index(),
			button_name: {
				if a.has_button_name() {
					Some(a.get_button_name().to_string())
				} else {
					None
				}
			},
			friendly_name: {
				if a.has_friendly_name() {
					Some(a.get_friendly_name().to_string())
				} else {
					None
				}
			},
			hotkey: {
				if a.has_hotkey() {
					Some(a.get_hotkey().to_string())
				} else {
					None
				}
			},
			remaps_to_ability_id: AbilityId::from_u32(a.get_remaps_to_ability_id()),
			available: a.get_available(),
			target: AbilityTarget::from_proto(a.get_target()),
			allow_minimap: a.get_allow_minimap(),
			allow_autocast: a.get_allow_autocast(),
			is_building: a.get_is_building(),
			footprint_radius: {
				if a.has_footprint_radius() {
					Some(a.get_footprint_radius())
				} else {
					None
				}
			},
			is_instant_placement: a.get_is_instant_placement(),
			cast_range: {
				if a.has_cast_range() {
					Some(a.get_cast_range())
				} else {
					None
				}
			},
		})
	}
}

#[derive(Clone)]
pub struct UnitTypeData {
	pub id: UnitTypeId,
	pub name: String,
	pub available: bool,
	pub cargo_size: u32,
	pub mineral_cost: u32,
	pub vespene_cost: u32,
	pub food_required: f32,
	pub food_provided: f32,
	pub ability: Option<AbilityId>,
	pub race: Race,
	pub build_time: f32,
	pub has_vespene: bool,
	pub has_minerals: bool,
	pub sight_range: f32,
	pub tech_alias: Vec<UnitTypeId>,
	pub unit_alias: Option<UnitTypeId>,
	pub tech_requirement: Option<UnitTypeId>,
	pub require_attached: bool,
	pub attributes: Vec<Attribute>,
	pub movement_speed: f32,
	pub armor: f32,
	pub weapons: Vec<Weapon>,
}
impl TryFromProto<ProtoUnitTypeData> for UnitTypeData {
	fn try_from_proto(u: ProtoUnitTypeData) -> Option<Self> {
		Some(Self {
			id: match UnitTypeId::from_u32(u.get_unit_id()) {
				Some(id) => id,
				None => return None,
			},
			name: u.get_name().to_string(),
			available: u.get_available(),
			cargo_size: u.get_cargo_size(),
			mineral_cost: u.get_mineral_cost(),
			vespene_cost: u.get_vespene_cost(),
			food_required: u.get_food_required(),
			food_provided: u.get_food_provided(),
			ability: AbilityId::from_u32(u.get_ability_id()),
			race: Race::from_proto(u.get_race()),
			build_time: u.get_build_time(),
			has_vespene: u.get_has_vespene(),
			has_minerals: u.get_has_minerals(),
			sight_range: u.get_sight_range(),
			tech_alias: u
				.get_tech_alias()
				.iter()
				.filter_map(|a| UnitTypeId::from_u32(*a))
				.collect(),
			unit_alias: UnitTypeId::from_u32(u.get_unit_alias()),
			tech_requirement: UnitTypeId::from_u32(u.get_tech_requirement()),
			require_attached: u.get_require_attached(),
			attributes: u
				.get_attributes()
				.iter()
				.map(|&a| Attribute::from_proto(a))
				.collect(),
			movement_speed: u.get_movement_speed(),
			armor: u.get_armor(),
			weapons: u
				.get_weapons()
				.iter()
				.map(|w| Weapon::from_proto(w.clone()))
				.collect(),
		})
	}
}

#[derive(Clone)]
pub struct UpgradeData {
	pub id: UpgradeId,
	pub name: String,
	pub mineral_cost: u32,
	pub vespene_cost: u32,
	pub research_time: f32,
	pub ability: AbilityId,
}
impl TryFromProto<ProtoUpgradeData> for UpgradeData {
	fn try_from_proto(u: ProtoUpgradeData) -> Option<Self> {
		Some(Self {
			id: match UpgradeId::from_u32(u.get_upgrade_id()) {
				Some(id) => id,
				None => return None,
			},
			name: u.get_name().to_string(),
			mineral_cost: u.get_mineral_cost(),
			vespene_cost: u.get_vespene_cost(),
			research_time: u.get_research_time(),
			ability: match AbilityId::from_u32(u.get_ability_id()) {
				Some(id) => id,
				None => return None,
			},
		})
	}
}

#[derive(Clone)]
pub struct BuffData {
	pub id: BuffId,
	pub name: String,
}
impl TryFromProto<ProtoBuffData> for BuffData {
	fn try_from_proto(b: ProtoBuffData) -> Option<Self> {
		Some(Self {
			id: match BuffId::from_u32(b.get_buff_id()) {
				Some(id) => id,
				None => return None,
			},
			name: b.get_name().to_string(),
		})
	}
}

#[derive(Clone)]
pub struct EffectData {
	pub id: EffectId,
	pub name: String,
	pub friendly_name: String,
	pub radius: f32,
}
impl TryFromProto<ProtoEffectData> for EffectData {
	fn try_from_proto(e: ProtoEffectData) -> Option<Self> {
		Some(Self {
			id: match EffectId::from_u32(e.get_effect_id()) {
				Some(id) => id,
				None => return None,
			},
			name: e.get_name().to_string(),
			friendly_name: e.get_friendly_name().to_string(),
			radius: e.get_radius(),
		})
	}
}
