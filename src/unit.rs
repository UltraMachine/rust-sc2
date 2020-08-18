//! Stuff for convenient interaction with [`Unit`]s.
#![allow(missing_docs)]

use crate::{
	action::{Commander, Target},
	bot::{Locked, Reader, Rs, Rw},
	constants::{
		RaceValues, DAMAGE_BONUS_PER_UPGRADE, FRAMES_PER_SECOND, MISSED_WEAPONS, OFF_CREEP_SPEED_UPGRADES,
		SPEED_BUFFS, SPEED_ON_CREEP, SPEED_UPGRADES, WARPGATE_ABILITIES,
	},
	distance::Distance,
	game_data::{Attribute, GameData, TargetType, UnitTypeData, Weapon},
	game_state::Alliance,
	geometry::{Point2, Point3},
	ids::{AbilityId, BuffId, UnitTypeId, UpgradeId},
	pixel_map::{PixelMap, VisibilityMap},
	player::Race,
	FromProto, FromProtoData,
};
use num_traits::FromPrimitive;
use rustc_hash::{FxHashMap, FxHashSet};
use sc2_proto::raw::{
	CloakState as ProtoCloakState, DisplayType as ProtoDisplayType, Unit as ProtoUnit,
	UnitOrder_oneof_target as ProtoTarget,
};

#[derive(Default, Clone)]
pub(crate) struct DataForUnit {
	pub commander: Rw<Commander>,
	pub game_data: Rs<GameData>,
	pub techlab_tags: Rw<FxHashSet<u64>>,
	pub reactor_tags: Rw<FxHashSet<u64>>,
	pub race_values: Rs<RaceValues>,
	pub max_cooldowns: Rw<FxHashMap<UnitTypeId, f32>>,
	pub last_units_health: Rs<FxHashMap<u64, u32>>,
	pub abilities_units: Rs<FxHashMap<u64, FxHashSet<AbilityId>>>,
	pub upgrades: Rw<FxHashSet<UpgradeId>>,
	pub enemy_upgrades: Rw<FxHashSet<UpgradeId>>,
	pub creep: Rs<PixelMap>,
	pub visibility: Rs<VisibilityMap>,
	pub game_step: u32,
}

/// Weapon target used in [`calculate_weapon_stats`](Unit::calculate_weapon_stats).
pub enum CalcTarget<'a> {
	/// Specific unit.
	Unit(&'a Unit),
	/// Abstract target with given type and attributes.
	Abstract(TargetType, &'a [Attribute]),
}

pub(crate) type SharedUnitData = Rs<DataForUnit>;

/// Unit structure contains some raw data, helper methods for it's analysis
/// and some methods for actions execution.
#[derive(Clone)]
pub struct Unit {
	data: SharedUnitData,
	/// Allows unit to forcibly execute commands, ignoring spam filter.
	pub allow_spam: bool,

	/////////////////////////////////////////////////
	// Fields are populated based on type/alliance //
	/////////////////////////////////////////////////
	/// How unit is displayed (i.e. visibility of unit).
	pub display_type: DisplayType,
	/// Unit is owned, enemy or just neutral.
	pub alliance: Alliance,

	/// Unique and constant for each unit tag. Used to find exactly the same unit in bunch of [`Units`].
	/// See also [`find_tag`] and [`find_tags`].
	///
	/// [`Units`]: crate::units::Units
	/// [`find_tag`]: crate::units::Units::find_tag
	/// [`find_tags`]: crate::units::Units::find_tags
	pub tag: u64,
	/// The type of unit.
	pub type_id: UnitTypeId,
	/// Player id of the owner. Normally it should match your [`player_id`] for owned units
	/// and [`enemy_player_id`] for opponent's units.
	///
	/// [`player_id`]: crate::bot::Bot::player_id
	/// [`enemy_player_id`]: crate::bot::Bot::enemy_player_id
	pub owner: u32,
	/// Position on 2D grid.
	pub position: Point2,
	/// Position in 3D world space.
	pub position3d: Point3,
	/// Unit rotation angle (i.e. the direction unit is facing).
	/// Value in range `[0, 2π)`.
	pub facing: f32,
	/// Radius of the unit.
	pub radius: f32,
	/// The progress of building construction. Value from `0` to `1`.
	pub build_progress: f32,
	/// Cloak state of unit. Used in [`is_cloaked`], [`is_revealed`], [`can_be_attacked`].
	///
	/// [`is_cloaked`]: Self::is_cloaked
	/// [`is_revealed`]: Self::is_revealed
	/// [`can_be_attacked`]: Self::can_be_attacked
	pub cloak: CloakState,
	/// Set of buffs unit has.
	pub buffs: FxHashSet<BuffId>,
	/// Detection range of detector or `0` if unit is not detector.
	/// See also [`is_detector`](Self::is_detector).
	pub detect_range: f32,
	/// Range of terran's sensor tower.
	pub radar_range: f32,
	/// Unit is selected.
	pub is_selected: bool,
	/// Unit is visible in game window.
	pub is_on_screen: bool,
	/// Enemies detected by sensor tower.
	pub is_blip: bool,
	/// Protoss structure is powered by pylon.
	pub is_powered: bool,
	/// Building is training/researching (i.e. animated).
	pub is_active: bool,
	/// General attack upgrade level without considering buffs and special upgrades.
	pub attack_upgrade_level: u32,
	/// General armor upgrade level without considering buffs and special upgrades.
	pub armor_upgrade_level: i32,
	/// General shield upgrade level without considering buffs and special upgrades.
	pub shield_upgrade_level: i32,

	/////////////////////////////////
	// Not populated for snapshots //
	/////////////////////////////////
	/// Current health of unit.
	///
	/// Note: Not populated for snapshots.
	pub health: Option<u32>,
	/// Maximum health of unit.
	///
	/// Note: Not populated for snapshots.
	pub health_max: Option<u32>,
	/// Current shield of protoss unit.
	///
	/// Note: Not populated for snapshots.
	pub shield: Option<u32>,
	/// Maximum shield of protoss unit.
	///
	/// Note: Not populated for snapshots.
	pub shield_max: Option<u32>,
	/// Current energy of caster unit.
	///
	/// Note: Not populated for snapshots.
	pub energy: Option<u32>,
	/// Maximum energy of caster unit.
	///
	/// Note: Not populated for snapshots.
	pub energy_max: Option<u32>,
	/// Amount of minerals left in mineral field.
	///
	/// Note: Not populated for snapshots.
	pub mineral_contents: Option<u32>,
	/// Amount of vespene gas left in vespene geyser.
	///
	/// Note: Not populated for snapshots.
	pub vespene_contents: Option<u32>,
	/// Unit is flying.
	///
	/// Note: Not populated for snapshots.
	pub is_flying: bool,
	/// Zerg unit is burrowed.
	///
	/// Note: Not populated for snapshots.
	pub is_burrowed: bool,
	/// Is hallucination created by protoss sentry.
	///
	/// Note: Not populated for snapshots.
	pub is_hallucination: bool,

	///////////////////////////////
	// Not populated for enemies //
	///////////////////////////////
	/// Current orders of unit.
	///
	/// Note: Not populated for enemies and snapshots.
	pub orders: Vec<UnitOrder>,
	/// Tag of addon if any.
	///
	/// Note: Not populated for enemies and snapshots.
	pub addon_tag: Option<u64>,
	/// Units inside transport or bunker.
	///
	/// Note: Not populated for enemies and snapshots.
	pub passengers: Vec<PassengerUnit>,
	/// Used space of transport or bunker.
	///
	/// Note: Not populated for enemies and snapshots.
	pub cargo_space_taken: Option<u32>,
	/// Maximum space of transport or bunker.
	///
	/// Note: Not populated for enemies and snapshots.
	pub cargo_space_max: Option<u32>,
	/// Current number of workers on gas or base.
	///
	/// Note: Not populated for enemies and snapshots.
	pub assigned_harvesters: Option<u32>,
	/// Ideal number of workers on gas or base.
	///
	/// Note: Not populated for enemies and snapshots.
	pub ideal_harvesters: Option<u32>,
	/// Frames left until weapon will be ready to shot.
	///
	/// Note: Not populated for enemies and snapshots.
	pub weapon_cooldown: Option<f32>,
	pub engaged_target_tag: Option<u64>,
	/// How long a buff or unit is still around (e.g. mule, broodling, chronoboost).
	///
	/// Note: Not populated for enemies and snapshots.
	pub buff_duration_remain: Option<u32>,
	/// How long the maximum duration of buff or unit (e.g. mule, broodling, chronoboost).
	///
	/// Note: Not populated for enemies and snapshots.
	pub buff_duration_max: Option<u32>,
	/// All rally points of structure.
	///
	/// Note: Not populated for enemies and snapshots.
	pub rally_targets: Vec<RallyTarget>,
}

impl Unit {
	fn type_data(&self) -> Option<&UnitTypeData> {
		self.data.game_data.units.get(&self.type_id)
	}
	fn upgrades(&self) -> Reader<FxHashSet<UpgradeId>> {
		if self.is_mine() {
			self.data.upgrades.lock_read()
		} else {
			self.data.enemy_upgrades.lock_read()
		}
	}
	/// Checks if unit is worker.
	pub fn is_worker(&self) -> bool {
		self.type_id.is_worker()
	}
	/// Checks if it's townhall.
	pub fn is_townhall(&self) -> bool {
		self.type_id.is_townhall()
	}
	/// Checks if it's addon.
	pub fn is_addon(&self) -> bool {
		self.type_id.is_addon()
	}
	/// Checks if unit is melee attacker.
	pub fn is_melee(&self) -> bool {
		self.type_id.is_melee()
	}
	/// Checks if it's mineral field.
	pub fn is_mineral(&self) -> bool {
		self.type_data().map_or(false, |data| data.has_minerals)
	}
	/// Checks if it's vespene geyser.
	pub fn is_geyser(&self) -> bool {
		self.type_data().map_or(false, |data| data.has_vespene)
	}
	/// Checks if unit is detector.
	#[rustfmt::skip::macros(matches)]
	pub fn is_detector(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::Observer
				| UnitTypeId::ObserverSiegeMode
				| UnitTypeId::Raven
				| UnitTypeId::Overseer
				| UnitTypeId::OverseerSiegeMode
		) || (self.is_ready()
			&& (matches!(self.type_id, UnitTypeId::MissileTurret | UnitTypeId::SporeCrawler)
				|| (matches!(self.type_id, UnitTypeId::PhotonCannon) && self.is_powered)))
	}
	/// Building construction complete.
	pub fn is_ready(&self) -> bool {
		(self.build_progress - 1.0).abs() < std::f32::EPSILON
	}
	/// Terran building has addon.
	pub fn has_addon(&self) -> bool {
		self.addon_tag.is_some()
	}
	/// Terran building's addon is techlab if any.
	pub fn has_techlab(&self) -> bool {
		let techlab_tags = self.data.techlab_tags.lock_read();
		self.addon_tag.map_or(false, |tag| techlab_tags.contains(&tag))
	}
	/// Terran building's addon is reactor if any.
	pub fn has_reactor(&self) -> bool {
		let reactor_tags = self.data.reactor_tags.lock_read();
		self.addon_tag.map_or(false, |tag| reactor_tags.contains(&tag))
	}
	/// Unit was attacked on last step.
	pub fn is_attacked(&self) -> bool {
		self.hits() < self.data.last_units_health.get(&self.tag).copied()
	}
	/// The damage was taken by unit if it was attacked, otherwise it's `0`.
	pub fn damage_taken(&self) -> u32 {
		let hits = match self.hits() {
			Some(hits) => hits,
			None => return 0,
		};
		let last_hits = match self.data.last_units_health.get(&self.tag) {
			Some(hits) => hits,
			None => return 0,
		};
		last_hits.saturating_sub(hits)
	}
	/// Abilities available for unit to use.
	///
	/// Ability won't be avaliable if it's on cooldown, unit
	/// is out of energy or bot hasn't got enough resources.
	pub fn abilities(&self) -> Option<&FxHashSet<AbilityId>> {
		self.data.abilities_units.get(&self.tag)
	}
	/// Checks if ability is available for unit.
	///
	/// Ability won't be avaliable if it's on cooldown, unit
	/// is out of energy or bot hasn't got enough resources.
	pub fn has_ability(&self, ability: AbilityId) -> bool {
		self.data
			.abilities_units
			.get(&self.tag)
			.map_or(false, |abilities| abilities.contains(&ability))
	}
	/// Race of unit, dependent on it's type.
	pub fn race(&self) -> Race {
		self.type_data().map_or(Race::Random, |data| data.race)
	}
	/// There're some units inside transport or bunker.
	pub fn has_cargo(&self) -> bool {
		self.cargo_space_taken.map_or(false, |taken| taken > 0)
	}
	/// Free space left in transport or bunker.
	pub fn cargo_left(&self) -> Option<u32> {
		Some(self.cargo_space_max? - self.cargo_space_taken?)
	}
	/// Half of [`building_size`](Self::building_size), but `2.5` for addons.
	pub fn footprint_radius(&self) -> Option<f32> {
		self.type_data().and_then(|data| {
			data.ability.and_then(|ability| {
				self.data
					.game_data
					.abilities
					.get(&ability)
					.and_then(|ability_data| ability_data.footprint_radius)
			})
		})
	}
	/// Correct building size in tiles
	/// (e.g. `2` for supply and addons, `3` for barracks, `5` for command center).
	pub fn building_size(&self) -> Option<usize> {
		if self.is_addon() {
			Some(2)
		} else {
			self.footprint_radius().map(|radius| (radius * 2.0) as usize)
		}
	}
	/// Space that unit takes in transports and bunkers.
	pub fn cargo_size(&self) -> u32 {
		self.type_data().map_or(0, |data| data.cargo_size)
	}
	/// How far unit can see.
	pub fn sight_range(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.sight_range)
	}
	/// Initial armor of unit without considering upgrades and buffs.
	pub fn armor(&self) -> i32 {
		self.type_data().map_or(0, |data| data.armor)
	}
	/// Returns point with given offset towards unit face direction.
	pub fn towards_facing(&self, offset: f32) -> Point2 {
		self.position
			.offset(offset * self.facing.cos(), offset * self.facing.sin())
	}
	#[inline]
	fn is_pos_visible(&self) -> bool {
		self.data.visibility[self.position].is_visible()
	}
	/// Checks if unit is fully visible.
	pub fn is_visible(&self) -> bool {
		self.display_type.is_visible() && self.is_pos_visible()
	}
	/// Checks if unit is snapshot (i.e. hidden in fog of war or on high ground).
	pub fn is_snapshot(&self) -> bool {
		self.display_type.is_snapshot() && !self.is_pos_visible()
	}
	/// Checks if unit is fully hidden.
	pub fn is_hidden(&self) -> bool {
		self.display_type.is_hidden()
	}
	/// Checks if unit is building placeholder.
	pub fn is_placeholder(&self) -> bool {
		self.display_type.is_placeholder()
	}
	/// Checks if unit is owned.
	pub fn is_mine(&self) -> bool {
		self.alliance.is_mine()
	}
	/// Checks if unit is enemy.
	pub fn is_enemy(&self) -> bool {
		self.alliance.is_enemy()
	}
	/// Checks if unit is neutral.
	pub fn is_neutral(&self) -> bool {
		self.alliance.is_neutral()
	}
	/// Checks if unit is allied, but not owned.
	pub fn is_ally(&self) -> bool {
		self.alliance.is_ally()
	}
	/// Checks if unit has cloak field turned on.
	pub fn is_cloaked(&self) -> bool {
		matches!(
			self.cloak,
			CloakState::Cloaked | CloakState::CloakedDetected | CloakState::CloakedAllied
		)
	}
	/// Checks if unit is cloaked, but detected.
	pub fn is_revealed(&self) -> bool {
		matches!(self.cloak, CloakState::CloakedDetected)
	}
	/// Checks if unit is detected or not even cloaked.
	pub fn can_be_attacked(&self) -> bool {
		matches!(self.cloak, CloakState::NotCloaked | CloakState::CloakedDetected)
	}
	/// Returns how much supply this unit uses.
	pub fn supply_cost(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.food_required)
	}
	/// Returns health percentage (current health divided by max health).
	/// Value in range from `0` to `1`.
	pub fn health_percentage(&self) -> Option<f32> {
		let current = self.health?;
		let max = self.health_max?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Returns shield percentage (current shield divided by max shield).
	/// Value in range from `0` to `1`.
	pub fn shield_percentage(&self) -> Option<f32> {
		let current = self.shield?;
		let max = self.shield_max?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Returns energy percentage (current energy divided by max energy).
	/// Value in range from `0` to `1`.
	pub fn energy_percentage(&self) -> Option<f32> {
		let current = self.energy?;
		let max = self.energy_max?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Returns summed health and shield.
	///
	/// Not populated for snapshots.
	pub fn hits(&self) -> Option<u32> {
		match (self.health, self.shield) {
			(Some(health), Some(shield)) => Some(health + shield),
			(Some(health), None) => Some(health),
			(None, Some(shield)) => Some(shield),
			(None, None) => None,
		}
	}
	/// Returns summed max health and max shield.
	///
	/// Not populated for snapshots.
	pub fn hits_max(&self) -> Option<u32> {
		match (self.health_max, self.shield_max) {
			(Some(health), Some(shield)) => Some(health + shield),
			(Some(health), None) => Some(health),
			(None, Some(shield)) => Some(shield),
			(None, None) => None,
		}
	}
	/// Returns percentage of summed health and shield (current hits divided by max hits).
	/// Value in range from `0` to `1`.
	///
	/// Not populated for snapshots.
	pub fn hits_percentage(&self) -> Option<f32> {
		let current = self.hits()?;
		let max = self.hits_max()?;
		if max == 0 {
			return None;
		}
		Some(current as f32 / max as f32)
	}
	/// Basic speed of the unit without considering buffs and upgrades.
	///
	/// Use [`real_speed`](Self::real_speed) to get speed including buffs and upgrades.
	pub fn speed(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.movement_speed)
	}
	/// Returns actual speed of the unit calculated including buffs and upgrades.
	pub fn real_speed(&self) -> f32 {
		let mut speed = self.speed();
		let unit_type = self.type_id;

		// ---- Buffs ----
		// Ultralisk has passive ability "Frenzied" which makes it immune to speed altering buffs
		if unit_type != UnitTypeId::Ultralisk {
			for buff in &self.buffs {
				match buff {
					BuffId::MedivacSpeedBoost => return speed * 1.7,
					BuffId::VoidRaySwarmDamageBoost => return speed * 0.75,
					_ => {
						if let Some(increase) = SPEED_BUFFS.get(&buff) {
							speed *= increase;
						}
					}
				}
			}
		}

		// ---- Upgrades ----
		let upgrades = self.upgrades();
		if let Some((upgrade_id, increase)) = SPEED_UPGRADES.get(&unit_type) {
			if upgrades.contains(upgrade_id) {
				speed *= increase;
			}
		}

		// ---- Creep ----
		// On creep
		if self.data.creep[self.position].is_set() {
			if let Some(increase) = SPEED_ON_CREEP.get(&unit_type) {
				speed *= increase;
			}
		}
		// Off creep upgrades
		if !upgrades.is_empty() {
			if let Some((upgrade_id, increase)) = OFF_CREEP_SPEED_UPGRADES.get(&unit_type) {
				if upgrades.contains(upgrade_id) {
					speed *= increase;
				}
			}
		}

		speed
	}
	/// Distance unit can travel per one step.
	pub fn distance_per_step(&self) -> f32 {
		self.real_speed() / FRAMES_PER_SECOND * self.data.game_step as f32
	}
	/// Distance unit can travel until weapons be ready to fire.
	pub fn distance_to_weapon_ready(&self) -> f32 {
		self.real_speed() / FRAMES_PER_SECOND * self.weapon_cooldown.unwrap_or(0.0)
	}
	/// Attributes of unit, dependent on it's type.
	pub fn attributes(&self) -> &[Attribute] {
		self.type_data().map_or(&[], |data| data.attributes.as_slice())
	}
	/// Checks if unit has given attribute.
	pub fn has_attribute(&self, attribute: Attribute) -> bool {
		self.type_data()
			.map_or(false, |data| data.attributes.contains(&attribute))
	}
	/// Checks if unit has `Light` attribute.
	pub fn is_light(&self) -> bool {
		self.has_attribute(Attribute::Light)
	}
	/// Checks if unit has `Armored` attribute.
	pub fn is_armored(&self) -> bool {
		self.has_attribute(Attribute::Armored)
	}
	/// Checks if unit has `Biological` attribute.
	pub fn is_biological(&self) -> bool {
		self.has_attribute(Attribute::Biological)
	}
	/// Checks if unit has `Mechanical` attribute.
	pub fn is_mechanical(&self) -> bool {
		self.has_attribute(Attribute::Mechanical)
	}
	/// Checks if unit has `Robotic` attribute.
	pub fn is_robotic(&self) -> bool {
		self.has_attribute(Attribute::Robotic)
	}
	/// Checks if unit has `Psionic` attribute.
	pub fn is_psionic(&self) -> bool {
		self.has_attribute(Attribute::Psionic)
	}
	/// Checks if unit has `Massive` attribute.
	pub fn is_massive(&self) -> bool {
		self.has_attribute(Attribute::Massive)
	}
	/// Checks if unit has `Structure` attribute.
	pub fn is_structure(&self) -> bool {
		self.has_attribute(Attribute::Structure)
	}
	/// Checks if unit has `Hover` attribute.
	pub fn is_hover(&self) -> bool {
		self.has_attribute(Attribute::Hover)
	}
	/// Checks if unit has `Heroic` attribute.
	pub fn is_heroic(&self) -> bool {
		self.has_attribute(Attribute::Heroic)
	}
	/// Checks if unit has `Summoned` attribute.
	pub fn is_summoned(&self) -> bool {
		self.has_attribute(Attribute::Summoned)
	}
	/// Checks if unit has given buff.
	pub fn has_buff(&self, buff: BuffId) -> bool {
		self.buffs.contains(&buff)
	}
	/// Checks if unit has any from given buffs.
	pub fn has_any_buff<'a, B: IntoIterator<Item = &'a BuffId>>(&self, buffs: B) -> bool {
		buffs.into_iter().any(|b| self.buffs.contains(&b))
	}
	/// Checks if worker is carrying minerals.
	pub fn is_carrying_minerals(&self) -> bool {
		self.has_any_buff(&[
			BuffId::CarryMineralFieldMinerals,
			BuffId::CarryHighYieldMineralFieldMinerals,
		])
	}
	/// Checks if worker is carrying vespene gas
	/// (Currently not works if worker is carrying gas from rich vespene geyeser,
	/// because SC2 API is not providing this information).
	pub fn is_carrying_vespene(&self) -> bool {
		self.has_any_buff(&[
			BuffId::CarryHarvestableVespeneGeyserGas,
			BuffId::CarryHarvestableVespeneGeyserGasProtoss,
			BuffId::CarryHarvestableVespeneGeyserGasZerg,
		])
	}
	/// Checks if worker is carrying any resource
	/// (Currently not works if worker is carrying gas from rich vespene geyeser,
	/// because SC2 API is not providing this information)
	pub fn is_carrying_resource(&self) -> bool {
		self.is_carrying_minerals() || self.is_carrying_vespene()
	}

	#[inline]
	fn weapons(&self) -> &[Weapon] {
		match self.type_id {
			UnitTypeId::Changeling
			| UnitTypeId::ChangelingZealot
			| UnitTypeId::ChangelingMarineShield
			| UnitTypeId::ChangelingMarine
			| UnitTypeId::ChangelingZerglingWings
			| UnitTypeId::ChangelingZergling => &[],
			_ => self
				.type_data()
				.map(|data| data.weapons.as_slice())
				.filter(|weapons| !weapons.is_empty())
				.or_else(|| match self.type_id {
					UnitTypeId::BanelingBurrowed | UnitTypeId::BanelingCocoon => {
						MISSED_WEAPONS.get(&UnitTypeId::Baneling).map(|ws| ws.as_slice())
					}
					UnitTypeId::RavagerCocoon => self
						.data
						.game_data
						.units
						.get(&UnitTypeId::Ravager)
						.map(|data| data.weapons.as_slice()),
					unit_type => MISSED_WEAPONS.get(&unit_type).map(|ws| ws.as_slice()),
				})
				.unwrap_or_default(),
		}
	}
	/// Targets unit can attack if it has weapon.
	pub fn weapon_target(&self) -> Option<TargetType> {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return None;
		}

		let mut ground = false;
		let mut air = false;
		if weapons.iter().any(|w| match w.target {
			TargetType::Ground => {
				ground = true;
				ground && air
			}
			TargetType::Air => {
				air = true;
				ground && air
			}
			_ => true,
		}) || (ground && air)
		{
			Some(TargetType::Any)
		} else if ground {
			Some(TargetType::Ground)
		} else if air {
			Some(TargetType::Air)
		} else {
			None
		}
	}
	/// Checks if unit can attack at all (i.e. has weapons).
	pub fn can_attack(&self) -> bool {
		!self.weapons().is_empty()
	}
	/// Checks if unit can attack both air and ground targets.
	pub fn can_attack_both(&self) -> bool {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return false;
		}

		let mut ground = false;
		let mut air = false;
		weapons.iter().any(|w| match w.target {
			TargetType::Ground => {
				ground = true;
				ground && air
			}
			TargetType::Air => {
				air = true;
				ground && air
			}
			_ => true,
		}) || (ground && air)
	}
	/// Checks if unit can attack ground targets.
	pub fn can_attack_ground(&self) -> bool {
		self.weapons().iter().any(|w| !w.target.is_air())
	}
	/// Checks if unit can attack air targets.
	pub fn can_attack_air(&self) -> bool {
		self.weapons().iter().any(|w| !w.target.is_ground())
	}
	/// Checks if unit can attack given target.
	pub fn can_attack_unit(&self, target: &Unit) -> bool {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return false;
		}

		if target.type_id == UnitTypeId::Colossus {
			!weapons.is_empty()
		} else {
			let not_target = {
				if target.is_flying {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons.iter().any(|w| w.target != not_target)
		}
	}
	/// Checks if unit's weapon is on cooldown.
	pub fn on_cooldown(&self) -> bool {
		self.weapon_cooldown.map_or(false, |cool| cool > f32::EPSILON)
	}
	/// Returns max cooldown in frames for unit's weapon.
	pub fn max_cooldown(&self) -> Option<f32> {
		self.data.max_cooldowns.lock_read().get(&self.type_id).copied()
	}
	/// Returns ground range of unit's weapon without considering upgrades.
	/// Use [`real_ground_range`](Self::real_ground_range) to get range including upgrades.
	pub fn ground_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_air())
			.map_or(0.0, |w| w.range)
	}
	/// Returns air range of unit's weapon without considering upgrades.
	/// Use [`real_air_range`](Self::real_air_range) to get range including upgrades.
	pub fn air_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_ground())
			.map_or(0.0, |w| w.range)
	}
	/// Returns range of unit's weapon vs given target if unit can it, otherwise returns `0`.
	/// Doesn't consider upgrades, use [`real_range_vs`](Self::real_range_vs)
	/// instead to get range including upgrades.
	pub fn range_vs(&self, target: &Unit) -> f32 {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return 0.0;
		}

		if target.type_id == UnitTypeId::Colossus {
			weapons
				.iter()
				.map(|w| w.range)
				.max_by(|r1, r2| r1.partial_cmp(r2).unwrap())
				.unwrap_or(0.0)
		} else {
			let not_target = {
				if target.is_flying {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons
				.iter()
				.find(|w| w.target != not_target)
				.map_or(0.0, |w| w.range)
		}
	}
	/// Returns actual ground range of unit's weapon including upgrades.
	pub fn real_ground_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_air())
			.map_or(0.0, |w| {
				let upgrades = self.upgrades();
				match self.type_id {
					UnitTypeId::Hydralisk => {
						if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
							return w.range + 1.0;
						}
					}
					UnitTypeId::Phoenix => {
						if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
							return w.range + 2.0;
						}
					}
					UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
						if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
							return w.range + 1.0;
						}
					}
					_ => {}
				}
				w.range
			})
	}
	/// Returns actual air range of unit's weapon including upgrades.
	pub fn real_air_range(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_ground())
			.map_or(0.0, |w| {
				let upgrades = self.upgrades();
				match self.type_id {
					UnitTypeId::Hydralisk => {
						if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
							return w.range + 1.0;
						}
					}
					UnitTypeId::Phoenix => {
						if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
							return w.range + 2.0;
						}
					}
					UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
						if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
							return w.range + 1.0;
						}
					}
					_ => {}
				}
				w.range
			})
	}
	/// Returns actual range of unit's weapon vs given target if unit can attack it, otherwise returs `0`.
	/// Takes upgrades into account.
	pub fn real_range_vs(&self, target: &Unit) -> f32 {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return 0.0;
		}

		let extract_range = |w: &Weapon| {
			let upgrades = self.upgrades();
			match self.type_id {
				UnitTypeId::Hydralisk => {
					if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
						return w.range + 1.0;
					}
				}
				UnitTypeId::Phoenix => {
					if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
						return w.range + 2.0;
					}
				}
				UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
					if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
						return w.range + 1.0;
					}
				}
				_ => {}
			}
			w.range
		};

		if target.type_id == UnitTypeId::Colossus {
			weapons
				.iter()
				.map(extract_range)
				.max_by(|r1, r2| r1.partial_cmp(r2).unwrap())
				.unwrap_or(0.0)
		} else {
			let not_target = {
				if target.is_flying {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons
				.iter()
				.find(|w| w.target != not_target)
				.map_or(0.0, extract_range)
		}
	}
	/// Returns ground dps of unit's weapon without considering upgrades.
	/// Use [`real_ground_weapon`](Self::real_ground_weapon) to get dps including upgrades.
	pub fn ground_dps(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_air())
			.map_or(0.0, |w| w.damage as f32 * (w.attacks as f32) / w.speed)
	}
	/// Returns air dps of unit's weapon without considering upgrades.
	/// Use [`real_air_weapon`](Self::real_air_weapon) to get dps including upgrades.
	pub fn air_dps(&self) -> f32 {
		self.weapons()
			.iter()
			.find(|w| !w.target.is_ground())
			.map_or(0.0, |w| w.damage as f32 * (w.attacks as f32) / w.speed)
	}
	/// Returns dps of unit's weapon vs given target if unit can it, otherwise returns `0`.
	/// Doesn't consider upgrades, use [`real_weapon_vs`](Self::real_weapon_vs)
	/// instead to get dps including upgrades.
	pub fn dps_vs(&self, target: &Unit) -> f32 {
		let weapons = self.weapons();
		if weapons.is_empty() {
			return 0.0;
		}

		let extract_dps = |w: &Weapon| w.damage as f32 * (w.attacks as f32) / w.speed;

		if target.type_id == UnitTypeId::Colossus {
			weapons
				.iter()
				.map(extract_dps)
				.max_by(|d1, d2| d1.partial_cmp(d2).unwrap())
				.unwrap_or(0.0)
		} else {
			let not_target = {
				if target.is_flying {
					TargetType::Ground
				} else {
					TargetType::Air
				}
			};
			weapons
				.iter()
				.find(|w| w.target != not_target)
				.map_or(0.0, extract_dps)
		}
	}

	/// Returns (dps, range) of first unit's weapon including bonuses from buffs and upgrades.
	///
	/// If you need to get only real range of unit, use [`real_ground_range`], [`real_air_range`]
	/// or [`real_range_vs`] instead, because they're generally faster.
	///
	/// [`real_range_vs`]: Self::real_range_vs
	/// [`real_ground_range`]: Self::real_ground_range
	/// [`real_air_range`]: Self::real_air_range
	pub fn real_weapon(&self, attributes: &[Attribute]) -> (f32, f32) {
		self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Any, attributes))
	}
	/// Returns (dps, range) of unit's ground weapon including bonuses from buffs and upgrades.
	///
	/// If you need to get only real range of unit, use [`real_ground_range`](Self::real_ground_range)
	/// instead, because it's generally faster.
	pub fn real_ground_weapon(&self, attributes: &[Attribute]) -> (f32, f32) {
		self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Ground, attributes))
	}
	/// Returns (dps, range) of unit's air weapon including bonuses from buffs and upgrades.
	///
	/// If you need to get only real range of unit, use [`real_air_range`](Self::real_air_range)
	/// instead, because it's generally faster.
	pub fn real_air_weapon(&self, attributes: &[Attribute]) -> (f32, f32) {
		self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Air, attributes))
	}
	/// Returns (dps, range) of unit's weapon vs given target if unit can attack it, otherwise returs `(0, 0)`.
	/// Takes buffs and upgrades into account.
	///
	/// If you need to get only real range of unit, use [`real_range_vs`](Self::real_range_vs)
	/// instead, because it's generally faster.
	pub fn real_weapon_vs(&self, target: &Unit) -> (f32, f32) {
		self.calculate_weapon_stats(CalcTarget::Unit(target))
	}

	/// Returns (dps, range) of unit's weapon vs given abstract target
	/// if unit can attack it, otherwise returs `(0, 0)`.
	/// Abstract target is described by it's type (air or ground) and attributes (e.g. light, armored, ...).
	///
	/// If you need to get only real range of unit, use [`real_ground_range`], [`real_air_range`]
	/// or [`real_range_vs`] instead, because they're generally faster.
	///
	/// [`real_range_vs`]: Self::real_range_vs
	/// [`real_ground_range`]: Self::real_ground_range
	/// [`real_air_range`]: Self::real_air_range
	pub fn calculate_weapon_abstract(&self, target_type: TargetType, attributes: &[Attribute]) -> (f32, f32) {
		self.calculate_weapon_stats(CalcTarget::Abstract(target_type, attributes))
	}

	/// Returns (dps, range) of unit's weapon vs given target (can be unit or abstract)
	/// if unit can attack it, otherwise returs `(0, 0)`.
	///
	/// If you need to get only real range of unit, use [`real_ground_range`], [`real_air_range`]
	/// or [`real_range_vs`] instead, because they're generally faster.
	///
	/// [`real_range_vs`]: Self::real_range_vs
	/// [`real_ground_range`]: Self::real_ground_range
	/// [`real_air_range`]: Self::real_air_range
	#[allow(clippy::mut_range_bound)]
	pub fn calculate_weapon_stats(&self, target: CalcTarget) -> (f32, f32) {
		/*
		if matches!(self.type_id, UnitTypeId::Bunker) && self.is_mine() {
			return self
				.passengers
				.iter()
				.map(|passenger| (passenger.type_id).calculate_weapon(target))
				.sum();
		}
		*/

		let (upgrades, target_upgrades) = {
			let my_upgrade = self.data.upgrades.lock_read();
			let enemy_upgrades = self.data.enemy_upgrades.lock_read();
			if self.is_mine() {
				(my_upgrade, enemy_upgrades)
			} else {
				(enemy_upgrades, my_upgrade)
			}
		};

		let (not_target, attributes, target_unit) = match target {
			CalcTarget::Unit(target) => {
				let mut enemy_armor = target.armor() + target.armor_upgrade_level;
				let mut enemy_shield_armor = target.shield_upgrade_level;

				let mut target_has_guardian_shield = false;

				target.buffs.iter().for_each(|buff| match buff {
					BuffId::GuardianShield => target_has_guardian_shield = true,
					_ => {
						#[cfg(windows)]
						const ANTI_ARMOR_BUFF: BuffId = BuffId::RavenShredderMissileArmorReductionUISubtruct;
						#[cfg(unix)]
						const ANTI_ARMOR_BUFF: BuffId = BuffId::RavenShredderMissileArmorReduction;

						if *buff == ANTI_ARMOR_BUFF {
							enemy_armor -= 3;
							enemy_shield_armor -= 3;
						}
					}
				});

				if !target_upgrades.is_empty() {
					if target.race().is_terran() {
						if target.is_structure() && target_upgrades.contains(&UpgradeId::TerranBuildingArmor)
						{
							enemy_armor += 2;
						}
					} else if matches!(
						target.type_id,
						UnitTypeId::Ultralisk | UnitTypeId::UltraliskBurrowed
					) && target_upgrades.contains(&UpgradeId::ChitinousPlating)
					{
						enemy_armor += 2;
					}
				}

				(
					if matches!(target.type_id, UnitTypeId::Colossus) {
						TargetType::Any
					} else if target.is_flying {
						TargetType::Ground
					} else {
						TargetType::Air
					},
					target.attributes(),
					Some((
						target,
						enemy_armor,
						enemy_shield_armor,
						target_has_guardian_shield,
					)),
				)
			}
			CalcTarget::Abstract(target_type, attributes) => (
				match target_type {
					TargetType::Any => TargetType::Any,
					TargetType::Ground => TargetType::Air,
					TargetType::Air => TargetType::Ground,
				},
				attributes,
				None,
			),
		};

		let weapons = self.weapons();
		if weapons.is_empty() {
			return (0.0, 0.0);
		}

		let mut speed_modifier = 1.0;
		let mut range_modifier = 0.0;

		self.buffs.iter().for_each(|buff| match buff {
			BuffId::Stimpack | BuffId::StimpackMarauder => speed_modifier /= 1.5,
			BuffId::TimeWarpProduction => speed_modifier *= 2.0,
			_ => {}
		});

		if !upgrades.is_empty() {
			match self.type_id {
				UnitTypeId::Zergling => {
					if upgrades.contains(&UpgradeId::Zerglingattackspeed) {
						speed_modifier /= 1.4;
					}
				}
				UnitTypeId::Adept => {
					if upgrades.contains(&UpgradeId::AdeptPiercingAttack) {
						speed_modifier /= 1.45;
					}
				}
				UnitTypeId::Hydralisk => {
					if upgrades.contains(&UpgradeId::EvolveGroovedSpines) {
						range_modifier += 1.0;
					}
				}
				UnitTypeId::Phoenix => {
					if upgrades.contains(&UpgradeId::PhoenixRangeUpgrade) {
						range_modifier += 2.0;
					}
				}
				UnitTypeId::PlanetaryFortress | UnitTypeId::MissileTurret | UnitTypeId::AutoTurret => {
					if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
						range_modifier += 1.0;
					}
				}
				_ => {}
			}
		}

		let damage_bonus_per_upgrade = DAMAGE_BONUS_PER_UPGRADE.get(&self.type_id);
		let extract_weapon_stats = |w: &Weapon| {
			let damage_bonus_per_upgrade = damage_bonus_per_upgrade.and_then(|bonus| bonus.get(&w.target));

			let mut damage = w.damage
				+ (self.attack_upgrade_level
					* damage_bonus_per_upgrade.and_then(|bonus| bonus.0).unwrap_or(1));
			let speed = w.speed * speed_modifier;
			let range = w.range + range_modifier;

			// Bonus damage
			if let Some(bonus) = w
				.damage_bonus
				.iter()
				.filter_map(|(attribute, bonus)| {
					if attributes.contains(attribute) {
						let mut damage_bonus_per_upgrade = damage_bonus_per_upgrade
							.and_then(|bonus| bonus.1.get(attribute))
							.copied()
							.unwrap_or(0);

						if let Attribute::Light = attribute {
							if upgrades.contains(&UpgradeId::HighCapacityBarrels) {
								match self.type_id {
									UnitTypeId::Hellion => damage_bonus_per_upgrade += 5,
									UnitTypeId::HellionTank => damage_bonus_per_upgrade += 12,
									_ => {}
								}
							}
						}

						let mut bonus_damage = bonus + (self.attack_upgrade_level * damage_bonus_per_upgrade);

						if let Attribute::Armored = attribute {
							if self.has_buff(BuffId::VoidRaySwarmDamageBoost) {
								bonus_damage += 6;
							}
						}

						Some(bonus_damage)
					} else {
						None
					}
				})
				.max_by(|b1, b2| b1.partial_cmp(b2).unwrap())
			{
				damage += bonus;
			}

			// Subtract damage
			match target_unit {
				Some((target, enemy_armor, enemy_shield_armor, target_has_guardian_shield)) => {
					let mut attacks = w.attacks;
					let mut shield_damage = 0;
					let mut health_damage = 0;

					if let Some(enemy_shield) = target.shield.filter(|shield| shield > &0) {
						let enemy_shield_armor = if target_has_guardian_shield && range >= 2.0 {
							enemy_shield_armor + 2
						} else {
							enemy_shield_armor
						};
						let exact_damage = 1.max(damage as i32 - enemy_shield_armor) as u32;

						for _ in 0..attacks {
							if shield_damage >= enemy_shield {
								health_damage = shield_damage - enemy_shield;
								break;
							}
							shield_damage += exact_damage;
							attacks -= 1;
						}
					}

					if let Some(enemy_health) = target.health.filter(|health| health > &0) {
						let enemy_armor = if target_has_guardian_shield && range >= 2.0 {
							enemy_armor + 2
						} else {
							enemy_armor
						};
						let exact_damage = 1.max(damage as i32 - enemy_armor) as u32;

						for _ in 0..attacks {
							if health_damage >= enemy_health {
								break;
							}
							health_damage += exact_damage;
						}
					}

					(shield_damage + health_damage, speed, range)
				}
				None => (damage * w.attacks, speed, range),
			}
		};
		let (damage, speed, range) = if not_target.is_any() {
			weapons
				.iter()
				.map(extract_weapon_stats)
				.max_by_key(|k| k.0)
				.unwrap_or((0, 0.0, 0.0))
		} else {
			weapons
				.iter()
				.filter(|w| w.target != not_target)
				.map(extract_weapon_stats)
				.max_by_key(|k| k.0)
				.unwrap_or((0, 0.0, 0.0))
		};
		(if speed == 0.0 { 0.0 } else { damage as f32 / speed }, range)
	}

	/// Checks if unit is close enough to attack given target.
	///
	/// See also [`in_real_range`](Self::in_real_range) which uses actual range of unit for calculations.
	pub fn in_range(&self, target: &Unit, gap: f32) -> bool {
		let range = {
			if matches!(target.type_id, UnitTypeId::Colossus) {
				match self
					.weapons()
					.iter()
					.map(|w| w.range)
					.max_by(|r1, r2| r1.partial_cmp(r2).unwrap())
				{
					Some(max_range) => max_range,
					None => return false,
				}
			} else {
				let range = if target.is_flying {
					self.air_range()
				} else {
					self.ground_range()
				};
				if range < f32::EPSILON {
					return false;
				}
				range
			}
		};
		let total_range = self.radius + target.radius + range + gap;
		let distance = self.distance_squared(target);

		// Takes into account that Sieged Tank has a minimum range of 2
		(self.type_id != UnitTypeId::SiegeTankSieged || distance > 4.0)
			&& distance <= total_range * total_range
	}
	/// Checks if unit is close enough to be attacked by given threat.
	/// This `unit.in_range_of(threat, gap)` is equivalent to `threat.in_range(unit, gap)`.
	///
	/// See also [`in_real_range_of`](Self::in_real_range_of) which uses actual range of unit for calculation.
	pub fn in_range_of(&self, threat: &Unit, gap: f32) -> bool {
		threat.in_range(self, gap)
	}
	/// Checks if unit is close enough to attack given target.
	///
	/// Uses actual range from [`real_range_vs`](Self::real_range_vs) in it's calculations.
	pub fn in_real_range(&self, target: &Unit, gap: f32) -> bool {
		let range = self.real_range_vs(target);
		if range < f32::EPSILON {
			return false;
		}

		let total_range = self.radius + target.radius + range + gap;
		let distance = self.distance_squared(target);

		// Takes into account that Sieged Tank has a minimum range of 2
		(self.type_id != UnitTypeId::SiegeTankSieged || distance > 4.0)
			&& distance <= total_range * total_range
	}
	/// Checks if unit is close enough to be attacked by given threat.
	/// This `unit.in_real_range_of(threat, gap)` is equivalent to `threat.in_real_range(unit, gap)`.
	///
	/// Uses actual range from [`real_range_vs`](Self::real_range_vs) in it's calculations.
	pub fn in_real_range_of(&self, threat: &Unit, gap: f32) -> bool {
		threat.in_real_range(self, gap)
	}
	/// Returns (attribute, bonus damage) for first unit's weapon if any.
	pub fn damage_bonus(&self) -> Option<(Attribute, u32)> {
		self.weapons()
			.iter()
			.find(|w| !w.damage_bonus.is_empty())
			.map(|w| w.damage_bonus[0])
	}
	/// Returns target of first unit's order.
	pub fn target(&self) -> Target {
		if self.is_idle() {
			Target::None
		} else {
			self.orders[0].target
		}
	}
	/// Returns target point of unit's order if any.
	pub fn target_pos(&self) -> Option<Point2> {
		match self.target() {
			Target::Pos(pos) => Some(pos),
			_ => None,
		}
	}
	/// Returns target tag of unit's order if any.
	pub fn target_tag(&self) -> Option<u64> {
		match self.target() {
			Target::Tag(tag) => Some(tag),
			_ => None,
		}
	}
	/// Returns ability of first unit's order.
	pub fn ordered_ability(&self) -> Option<AbilityId> {
		if self.is_idle() {
			None
		} else {
			Some(self.orders[0].ability)
		}
	}
	/// Checks if unit don't have any orders currently.
	pub fn is_idle(&self) -> bool {
		self.orders.is_empty()
	}
	/// Checks if unit don't have any orders currently or it's order is more than 95% complete.
	pub fn is_almost_idle(&self) -> bool {
		self.is_idle() || (self.orders.len() == 1 && self.orders[0].progress >= 0.95)
	}
	/// Checks if production building with reactor don't have any orders currently.
	pub fn is_unused(&self) -> bool {
		if self.has_reactor() {
			self.orders.len() < 2
		} else {
			self.is_idle()
		}
	}
	/// Checks if production building with reactor don't have any orders currently
	/// or it's order is more than 95% complete.
	pub fn is_almost_unused(&self) -> bool {
		if self.has_reactor() {
			self.orders.len() < 2
				|| (self.orders.len() == 2 && self.orders.iter().any(|order| order.progress >= 0.95))
		} else {
			self.is_almost_idle()
		}
	}
	/// Checks if unit is using given ability.
	///
	/// Doesn't work with enemies.
	pub fn is_using(&self, ability: AbilityId) -> bool {
		!self.is_idle() && self.orders[0].ability == ability
	}
	/// Checks if unit is using any of given abilities.
	///
	/// Doesn't work with enemies.
	pub fn is_using_any<'a, A: IntoIterator<Item = &'a AbilityId>>(&self, abilities: A) -> bool {
		!self.is_idle() && abilities.into_iter().any(|a| self.orders[0].ability == *a)
	}
	/// Checks if unit is currently attacking.
	///
	/// Doesn't work with enemies.
	#[rustfmt::skip::macros(matches)]
	pub fn is_attacking(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::Attack
					| AbilityId::AttackAttack
					| AbilityId::AttackAttackTowards
					| AbilityId::AttackAttackBarrage
					| AbilityId::ScanMove
			)
	}
	/// Checks if unit is currently moving.
	///
	/// Doesn't work with enemies.
	pub fn is_moving(&self) -> bool {
		self.is_using(AbilityId::MoveMove)
	}
	/// Checks if unit is currently patrolling.
	///
	/// Doesn't work with enemies.
	pub fn is_patrolling(&self) -> bool {
		self.is_using(AbilityId::Patrol)
	}
	/// Checks if SCV or MULE is currently repairing.
	///
	/// Doesn't work with enemies.
	pub fn is_repairing(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::EffectRepair | AbilityId::EffectRepairSCV | AbilityId::EffectRepairMule
			)
	}
	/// Checks if worker is currently gathering resource.
	///
	/// Doesn't work with enemies.
	pub fn is_gathering(&self) -> bool {
		self.is_using(AbilityId::HarvestGather)
	}
	/// Checks if worker is currently returning resource closest base.
	///
	/// Doesn't work with enemies.
	pub fn is_returning(&self) -> bool {
		self.is_using(AbilityId::HarvestReturn)
	}
	/// Checks if worker is currently gathering or returning resources.
	///
	/// Doesn't work with enemies.
	pub fn is_collecting(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::HarvestGather | AbilityId::HarvestReturn
			)
	}
	/// Checks if worker is currently constructing a building.
	///
	/// Doesn't work with enemies.
	pub fn is_constructing(&self) -> bool {
		!self.is_idle() && self.orders[0].ability.is_constructing()
	}
	/// Checks if terran building is currently making addon.
	///
	/// Doesn't work with enemies.
	pub fn is_making_addon(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::BuildTechLabBarracks
					| AbilityId::BuildReactorBarracks
					| AbilityId::BuildTechLabFactory
					| AbilityId::BuildReactorFactory
					| AbilityId::BuildTechLabStarport
					| AbilityId::BuildReactorStarport
			)
	}
	/// Checks if terran building is currently building techlab.
	///
	/// Doesn't work with enemies.
	pub fn is_making_techlab(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::BuildTechLabBarracks
					| AbilityId::BuildTechLabFactory
					| AbilityId::BuildTechLabStarport
			)
	}
	/// Checks if terran building is currently building reactor.
	///
	/// Doesn't work with enemies.
	pub fn is_making_reactor(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::BuildReactorBarracks
					| AbilityId::BuildReactorFactory
					| AbilityId::BuildReactorStarport
			)
	}

	// Actions

	/// Toggles autocast on given ability.
	pub fn toggle_autocast(&self, ability: AbilityId) {
		self.data
			.commander
			.lock_write()
			.autocast
			.entry(ability)
			.or_default()
			.push(self.tag);
	}
	/// Orders unit to execute given command.
	pub fn command(&self, ability: AbilityId, target: Target, queue: bool) {
		if !(queue || self.allow_spam || self.is_idle()) {
			let last_order = &self.orders[0];
			if ability == last_order.ability && target == last_order.target {
				return;
			}
		}

		self.data
			.commander
			.lock_write()
			.commands
			.entry((ability, target, queue))
			.or_default()
			.push(self.tag);
	}
	/// Orders unit to use given ability (This is equivalent of `unit.command(ability, Target::None, queue)`).
	pub fn use_ability(&self, ability: AbilityId, queue: bool) {
		self.command(ability, Target::None, queue)
	}
	/// Orders unit a `Smart` ability (This is equivalent of right click).
	pub fn smart(&self, target: Target, queue: bool) {
		self.command(AbilityId::Smart, target, queue)
	}
	/// Orders unit to attack given target.
	pub fn attack(&self, target: Target, queue: bool) {
		self.command(AbilityId::Attack, target, queue)
	}
	/// Orders unit to move to given target.
	pub fn move_to(&self, target: Target, queue: bool) {
		self.command(AbilityId::MoveMove, target, queue)
	}
	/// Orders unit to hold position.
	pub fn hold_position(&self, queue: bool) {
		self.command(AbilityId::HoldPosition, Target::None, queue)
	}
	/// Orders worker to gather given resource.
	pub fn gather(&self, target: u64, queue: bool) {
		self.command(AbilityId::HarvestGather, Target::Tag(target), queue)
	}
	/// Orders worker to return resource to closest base.
	pub fn return_resource(&self, queue: bool) {
		self.command(AbilityId::HarvestReturn, Target::None, queue)
	}
	/// Orders unit to stop actions.
	pub fn stop(&self, queue: bool) {
		self.command(AbilityId::Stop, Target::None, queue)
	}
	/// Orders unit to patrol.
	pub fn patrol(&self, target: Target, queue: bool) {
		self.command(AbilityId::Patrol, target, queue)
	}
	/// Orders SCV or MULE to repair given structure or mechanical unit.
	pub fn repair(&self, target: u64, queue: bool) {
		self.command(AbilityId::EffectRepair, Target::Tag(target), queue)
	}
	/// Orders building which is in progress to cancel construction.
	pub fn cancel_building(&self, queue: bool) {
		self.command(AbilityId::CancelBuildInProgress, Target::None, queue)
	}
	/// Orders production building to cancel last unit in train queue.
	pub fn cancel_queue(&self, queue: bool) {
		self.command(
			if self.is_townhall() {
				AbilityId::CancelQueueCancelToSelection
			} else {
				AbilityId::CancelQueue5
			},
			Target::None,
			queue,
		)
	}
	/// Orders worker to build race gas building on given geyser.
	pub fn build_gas(&self, target: u64, queue: bool) {
		self.command(
			self.data.game_data.units[&self.data.race_values.gas]
				.ability
				.unwrap(),
			Target::Tag(target),
			queue,
		)
	}
	/// Orders worker to build something on given position.
	pub fn build(&self, unit: UnitTypeId, target: Point2, queue: bool) {
		if let Some(type_data) = self.data.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				self.command(ability, Target::Pos(target), queue);
			}
		}
	}
	/// Orders production building to train given unit.
	///
	/// This also works for morphing units and building addons.
	pub fn train(&self, unit: UnitTypeId, queue: bool) {
		if let Some(type_data) = self.data.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				self.command(ability, Target::None, queue);
			}
		}
	}
	/// Orders building to research given upgrade.
	pub fn research(&self, upgrade: UpgradeId, queue: bool) {
		match upgrade {
			UpgradeId::TerranVehicleAndShipArmorsLevel1
			| UpgradeId::TerranVehicleAndShipArmorsLevel2
			| UpgradeId::TerranVehicleAndShipArmorsLevel3 => self.command(
				AbilityId::ResearchTerranVehicleAndShipPlating,
				Target::None,
				queue,
			),
			_ => {
				if let Some(type_data) = self.data.game_data.upgrades.get(&upgrade) {
					self.command(type_data.ability, Target::None, queue);
				}
			}
		}
	}
	/// Orders protoss warp gate to warp unit on given position.
	pub fn warp_in(&self, unit: UnitTypeId, target: Point2) {
		if let Some(ability) = WARPGATE_ABILITIES.get(&unit) {
			self.command(*ability, Target::Pos(target), false);
		}
	}
}
impl From<Unit> for Point2 {
	#[inline]
	fn from(u: Unit) -> Self {
		u.position
	}
}
impl From<&Unit> for Point2 {
	#[inline]
	fn from(u: &Unit) -> Self {
		u.position
	}
}

impl FromProtoData<&ProtoUnit> for Unit {
	fn from_proto_data(data: SharedUnitData, u: &ProtoUnit) -> Self {
		let pos = u.get_pos();
		let type_id = UnitTypeId::from_u32(u.get_unit_type()).unwrap();
		Self {
			data,
			allow_spam: false,
			display_type: DisplayType::from_proto(u.get_display_type()),
			alliance: Alliance::from_proto(u.get_alliance()),
			tag: u.get_tag(),
			type_id,
			owner: u.get_owner() as u32,
			position: Point2::from_proto(pos),
			position3d: Point3::from_proto(pos),
			facing: u.get_facing(),
			radius: u.get_radius(),
			build_progress: u.get_build_progress(),
			cloak: CloakState::from_proto(u.get_cloak()),
			buffs: u
				.get_buff_ids()
				.iter()
				.map(|b| BuffId::from_u32(*b).unwrap())
				.collect(),
			detect_range: match type_id {
				UnitTypeId::Observer => 11.0,
				UnitTypeId::ObserverSiegeMode => 13.75,
				_ => u.get_detect_range(),
			},
			radar_range: u.get_radar_range(),
			is_selected: u.get_is_selected(),
			is_on_screen: u.get_is_on_screen(),
			is_blip: u.get_is_blip(),
			is_powered: u.get_is_powered(),
			is_active: u.get_is_active(),
			attack_upgrade_level: u.get_attack_upgrade_level() as u32,
			armor_upgrade_level: u.get_armor_upgrade_level(),
			shield_upgrade_level: u.get_shield_upgrade_level(),
			// Not populated for snapshots
			health: u.health.map(|x| x as u32),
			health_max: u.health_max.map(|x| x as u32),
			shield: u.shield.map(|x| x as u32),
			shield_max: u.shield_max.map(|x| x as u32),
			energy: u.energy.map(|x| x as u32),
			energy_max: u.energy_max.map(|x| x as u32),
			mineral_contents: u.mineral_contents.map(|x| x as u32),
			vespene_contents: u.vespene_contents.map(|x| x as u32),
			is_flying: u.get_is_flying(),
			is_burrowed: u.get_is_burrowed(),
			is_hallucination: u.get_is_hallucination(),
			// Not populated for enemies
			orders: u
				.get_orders()
				.iter()
				.map(|order| UnitOrder {
					ability: AbilityId::from_u32(order.get_ability_id()).unwrap(),
					target: match &order.target {
						Some(ProtoTarget::target_world_space_pos(pos)) => {
							Target::Pos(Point2::from_proto(pos))
						}
						Some(ProtoTarget::target_unit_tag(tag)) => Target::Tag(*tag),
						None => Target::None,
					},
					progress: order.get_progress(),
				})
				.collect(),
			addon_tag: u.add_on_tag,
			passengers: u
				.get_passengers()
				.iter()
				.map(|p| PassengerUnit {
					tag: p.get_tag(),
					health: p.get_health(),
					health_max: p.get_health_max(),
					shield: p.get_shield(),
					shield_max: p.get_shield_max(),
					energy: p.get_energy(),
					energy_max: p.get_energy_max(),
					type_id: UnitTypeId::from_u32(p.get_unit_type()).unwrap(),
				})
				.collect(),
			cargo_space_taken: u.cargo_space_taken.map(|x| x as u32),
			cargo_space_max: u.cargo_space_max.map(|x| x as u32),
			assigned_harvesters: u.assigned_harvesters.map(|x| x as u32),
			ideal_harvesters: u.ideal_harvesters.map(|x| x as u32),
			weapon_cooldown: u.weapon_cooldown,
			engaged_target_tag: u.engaged_target_tag,
			buff_duration_remain: u.buff_duration_remain.map(|x| x as u32),
			buff_duration_max: u.buff_duration_max.map(|x| x as u32),
			rally_targets: u
				.get_rally_targets()
				.iter()
				.map(|t| RallyTarget {
					point: Point2::from_proto(t.get_point()),
					tag: t.tag,
				})
				.collect(),
		}
	}
}

/// The display type of [`Unit`]. Can be accessed through [`display_type`](Unit::display_type) field.
#[variant_checkers]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DisplayType {
	/// Fully visible.
	Visible,
	/// Dimmed version of unit left behind after entering fog of war.
	Snapshot,
	/// Fully hidden.
	Hidden,
	/// Building that hasn't started construction.
	Placeholder,
}

impl FromProto<ProtoDisplayType> for DisplayType {
	fn from_proto(display_type: ProtoDisplayType) -> Self {
		match display_type {
			ProtoDisplayType::Visible => DisplayType::Visible,
			ProtoDisplayType::Snapshot => DisplayType::Snapshot,
			ProtoDisplayType::Hidden => DisplayType::Hidden,
			ProtoDisplayType::Placeholder => DisplayType::Placeholder,
		}
	}
}

/// Cloak state of [`Unit`]. Can be accessed through [`cloak`](Unit::cloak) field.
#[derive(Clone, PartialEq, Eq)]
pub enum CloakState {
	/// Under the fog, so unknown whether it's cloaked or not.
	CloakedUnknown,
	/// Is cloaked (i.e. invisible).
	Cloaked,
	/// Is cloaked, but visible because is detected (i.e. in range of detector, or orbital scan).
	CloakedDetected,
	/// Unit is not cloaked.
	NotCloaked,
	/// Is cloaked, but visible because it's owned or allied unit.
	CloakedAllied,
}
impl FromProto<ProtoCloakState> for CloakState {
	fn from_proto(cloak_state: ProtoCloakState) -> Self {
		match cloak_state {
			ProtoCloakState::CloakedUnknown => CloakState::CloakedUnknown,
			ProtoCloakState::Cloaked => CloakState::Cloaked,
			ProtoCloakState::CloakedDetected => CloakState::CloakedDetected,
			ProtoCloakState::NotCloaked => CloakState::NotCloaked,
			ProtoCloakState::CloakedAllied => CloakState::CloakedAllied,
		}
	}
}

/// Order given to unit. All current orders of unit stored in [`orders`](Unit::orders) field.
#[derive(Clone)]
pub struct UnitOrder {
	/// Ability unit is using.
	pub ability: AbilityId,
	/// Target of unit's ability.
	pub target: Target,
	/// Progress of train abilities. Value in range from `0` to `1`.
	pub progress: f32,
}

/// Unit inside transport or bunker. All passengers stored in [`passengers`](Unit::passengers) field.
#[derive(Clone)]
pub struct PassengerUnit {
	pub tag: u64,
	pub health: f32,
	pub health_max: f32,
	pub shield: f32,
	pub shield_max: f32,
	pub energy: f32,
	pub energy_max: f32,
	pub type_id: UnitTypeId,
}

/// Rally point of production building. All rally points stored in [`rally_targets`](Unit::rally_targets) field.
#[derive(Clone)]
pub struct RallyTarget {
	/// Rally point. Position building rallied on.
	pub point: Point2,
	/// Filled if building is rallied on unit.
	pub tag: Option<u64>,
}
