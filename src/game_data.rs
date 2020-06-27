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
				.filter_map(|a| AbilityData::try_from_proto(a).map(|data| (data.id, data)))
				.collect(),
			units: data
				.get_units()
				.iter()
				.filter_map(|u| UnitTypeData::try_from_proto(u).map(|data| (data.id, data)))
				.collect(),
			upgrades: data
				.get_upgrades()
				.iter()
				.filter_map(|u| UpgradeData::try_from_proto(u).map(|data| (data.id, data)))
				.collect(),
			buffs: data
				.get_buffs()
				.iter()
				.filter_map(|b| BuffData::try_from_proto(b).map(|data| (data.id, data)))
				.collect(),
			effects: data
				.get_effects()
				.iter()
				.filter_map(|e| EffectData::try_from_proto(e).map(|data| (data.id, data)))
				.collect(),
		}
	}
}

#[derive(Debug, Default)]
pub struct Cost {
	pub minerals: u32,
	pub vespene: u32,
	pub supply: f32,
	pub time: f32,
}

#[derive(Copy, Clone)]
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

#[variant_checkers]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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

#[variant_checkers]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
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
impl FromProto<&ProtoWeapon> for Weapon {
	fn from_proto(weapon: &ProtoWeapon) -> Self {
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
impl TryFromProto<&ProtoAbilityData> for AbilityData {
	fn try_from_proto(a: &ProtoAbilityData) -> Option<Self> {
		Some(Self {
			id: AbilityId::from_u32(a.get_ability_id())?,
			link_name: a.get_link_name().to_string(),
			link_index: a.get_link_index(),
			button_name: a.button_name.as_ref().cloned(),
			friendly_name: a.friendly_name.as_ref().cloned(),
			hotkey: a.hotkey.as_ref().cloned(),
			remaps_to_ability_id: a.remaps_to_ability_id.and_then(AbilityId::from_u32),
			available: a.get_available(),
			target: AbilityTarget::from_proto(a.get_target()),
			allow_minimap: a.get_allow_minimap(),
			allow_autocast: a.get_allow_autocast(),
			is_building: a.get_is_building(),
			footprint_radius: a.footprint_radius,
			is_instant_placement: a.get_is_instant_placement(),
			cast_range: a.cast_range,
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
	pub armor: i32,
	pub weapons: Vec<Weapon>,
}
impl UnitTypeData {
	pub fn cost(&self) -> Cost {
		Cost {
			minerals: self.mineral_cost,
			vespene: self.vespene_cost,
			supply: self.food_required,
			time: self.build_time,
		}
	}
}
impl TryFromProto<&ProtoUnitTypeData> for UnitTypeData {
	fn try_from_proto(u: &ProtoUnitTypeData) -> Option<Self> {
		Some(Self {
			id: UnitTypeId::from_u32(u.get_unit_id())?,
			name: u.get_name().to_string(),
			available: u.get_available(),
			cargo_size: u.get_cargo_size(),
			mineral_cost: u.get_mineral_cost(),
			vespene_cost: u.get_vespene_cost(),
			food_required: u.get_food_required(),
			food_provided: u.get_food_provided(),
			ability: u.ability_id.and_then(AbilityId::from_u32),
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
			unit_alias: u.unit_alias.and_then(UnitTypeId::from_u32),
			tech_requirement: u.tech_requirement.and_then(UnitTypeId::from_u32),
			require_attached: u.get_require_attached(),
			attributes: u
				.get_attributes()
				.iter()
				.map(|&a| Attribute::from_proto(a))
				.collect(),
			movement_speed: u.get_movement_speed(),
			armor: u.get_armor() as i32,
			weapons: u.get_weapons().iter().map(|w| Weapon::from_proto(w)).collect(),
		})
	}
}

#[derive(Clone)]
pub struct UpgradeData {
	pub id: UpgradeId,
	pub ability: AbilityId,
	pub name: String,
	pub mineral_cost: u32,
	pub vespene_cost: u32,
	pub research_time: f32,
}
impl UpgradeData {
	pub fn cost(&self) -> Cost {
		Cost {
			minerals: self.mineral_cost,
			vespene: self.vespene_cost,
			supply: 0.0,
			time: self.research_time,
		}
	}
}
impl TryFromProto<&ProtoUpgradeData> for UpgradeData {
	fn try_from_proto(u: &ProtoUpgradeData) -> Option<Self> {
		Some(Self {
			id: UpgradeId::from_u32(u.get_upgrade_id())?,
			ability: AbilityId::from_u32(u.get_ability_id())?,
			name: u.get_name().to_string(),
			mineral_cost: u.get_mineral_cost(),
			vespene_cost: u.get_vespene_cost(),
			research_time: u.get_research_time(),
		})
	}
}

#[derive(Clone)]
pub struct BuffData {
	pub id: BuffId,
	pub name: String,
}
impl TryFromProto<&ProtoBuffData> for BuffData {
	fn try_from_proto(b: &ProtoBuffData) -> Option<Self> {
		Some(Self {
			id: BuffId::from_u32(b.get_buff_id())?,
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
	pub target: TargetType,
	pub friendly_fire: bool,
}
impl TryFromProto<&ProtoEffectData> for EffectData {
	fn try_from_proto(e: &ProtoEffectData) -> Option<Self> {
		EffectId::from_u32(e.get_effect_id()).map(|id| Self {
			id,
			name: e.get_name().to_string(),
			friendly_name: e.get_friendly_name().to_string(),
			radius: e.get_radius(),
			target: match id {
				EffectId::Null
				| EffectId::PsiStormPersistent
				| EffectId::ScannerSweep
				| EffectId::NukePersistent
				| EffectId::RavagerCorrosiveBileCP => TargetType::Any,
				_ => TargetType::Ground,
			},
			friendly_fire: match id {
				EffectId::PsiStormPersistent
				| EffectId::NukePersistent
				| EffectId::RavagerCorrosiveBileCP => true,
				_ => false,
			},
		})
	}
}
