use crate::{
	action::{Commander, Target},
	constants::{
		RaceValues, DAMAGE_BONUS_PER_UPGRADE, FRAMES_PER_SECOND, MISSED_WEAPONS, OFF_CREEP_SPEED_UPGRADES,
		SPEED_BUFFS, SPEED_ON_CREEP, SPEED_UPGRADES, TARGET_AIR, TARGET_GROUND, WARPGATE_ABILITIES,
	},
	game_data::{Attribute, GameData, TargetType, UnitTypeData, Weapon},
	game_state::Alliance,
	geometry::{Point2, Point3},
	ids::{AbilityId, BuffId, UnitTypeId, UpgradeId},
	pixel_map::PixelMap,
	player::Race,
	FromProto, FromProtoData,
};
use num_traits::FromPrimitive;
use sc2_proto::raw::{
	CloakState as ProtoCloakState, DisplayType as ProtoDisplayType, Unit as ProtoUnit,
	UnitOrder_oneof_target as ProtoTarget,
};
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Default, Clone)]
pub struct DataForUnit {
	pub commander: Rc<RefCell<Commander>>,
	pub game_data: Rc<GameData>,
	pub techlab_tags: Rc<RefCell<Vec<u64>>>,
	pub reactor_tags: Rc<RefCell<Vec<u64>>>,
	pub race_values: Rc<RaceValues>,
	pub max_cooldowns: Rc<RefCell<HashMap<UnitTypeId, f32>>>,
	pub upgrades: Rc<Vec<UpgradeId>>,
	pub creep: Rc<PixelMap>,
	pub game_step: u32,
}

pub enum CalcTarget<'a> {
	Unit(&'a Unit),
	Abstract(TargetType, Option<&'a Vec<Attribute>>),
}

#[derive(Clone)]
pub struct Unit {
	data: Rc<DataForUnit>,
	pub allow_spam: bool,

	// Fields are populated based on type/alliance
	pub display_type: DisplayType,
	pub alliance: Alliance,

	pub tag: u64,
	pub type_id: UnitTypeId,
	pub owner: u32,
	pub position: Point2,
	pub position3d: Point3,
	pub facing: f32, // Range 0..2pi
	pub radius: f32,
	pub build_progress: f32, // Range 0..1
	pub cloak: CloakState,
	pub buffs: Vec<BuffId>,
	pub detect_range: f32,
	pub radar_range: f32,
	pub is_selected: bool,
	pub is_on_screen: bool,
	pub is_blip: bool, // Detected by sensor tower
	pub is_powered: bool,
	pub is_active: bool, // Building is training/researching (ie animated).
	pub attack_upgrade_level: u32,
	pub armor_upgrade_level: i32,
	pub shield_upgrade_level: i32,

	// Not populated for snapshots
	pub health: Option<f32>,
	pub health_max: Option<f32>,
	pub shield: Option<f32>,
	pub shield_max: Option<f32>,
	pub energy: Option<f32>,
	pub energy_max: Option<f32>,
	pub mineral_contents: Option<u32>,
	pub vespene_contents: Option<u32>,
	pub is_flying: bool,
	pub is_burrowed: bool,
	pub is_hallucination: bool,

	// Not populated for enemies
	pub orders: Vec<UnitOrder>,
	pub add_on_tag: Option<u64>,
	pub passengers: Vec<PassengerUnit>,
	pub cargo_space_taken: Option<u32>,
	pub cargo_space_max: Option<u32>,
	pub assigned_harvesters: Option<u32>,
	pub ideal_harvesters: Option<u32>,
	pub weapon_cooldown: Option<f32>, // In frames
	pub engaged_target_tag: Option<u64>,
	pub buff_duration_remain: Option<u32>, // How long a buff or unit is still around (eg mule, broodling, chronoboost).
	pub buff_duration_max: Option<u32>, // How long the maximum duration of buff or unit (eg mule, broodling, chronoboost).
	pub rally_targets: Vec<RallyTarget>,
}

impl Unit {
	fn type_data(&self) -> Option<&UnitTypeData> {
		self.data.game_data.units.get(&self.type_id)
	}
	pub fn is_worker(&self) -> bool {
		self.type_id.is_worker()
	}
	pub fn is_townhall(&self) -> bool {
		self.type_id.is_townhall()
	}
	pub fn is_addon(&self) -> bool {
		self.type_id.is_addon()
	}
	pub fn is_melee(&self) -> bool {
		self.type_id.is_melee()
	}
	pub fn is_mineral(&self) -> bool {
		self.type_data().map_or(false, |data| data.has_minerals)
	}
	pub fn is_geyser(&self) -> bool {
		self.type_data().map_or(false, |data| data.has_vespene)
	}
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
	pub fn is_ready(&self) -> bool {
		(self.build_progress - 1.0).abs() < std::f32::EPSILON
	}
	pub fn has_add_on(&self) -> bool {
		self.add_on_tag.is_some()
	}
	pub fn has_techlab(&self) -> bool {
		self.add_on_tag
			.map_or(false, |tag| self.data.techlab_tags.borrow().contains(&tag))
	}
	pub fn has_reactor(&self) -> bool {
		self.add_on_tag
			.map_or(false, |tag| self.data.reactor_tags.borrow().contains(&tag))
	}
	pub fn race(&self) -> Race {
		self.type_data().map_or(Race::Random, |data| data.race)
	}
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
	pub fn building_size(&self) -> Option<usize> {
		if self.is_addon() {
			Some(2)
		} else {
			self.footprint_radius().map(|radius| (radius * 2.0) as usize)
		}
	}
	pub fn cargo_size(&self) -> u32 {
		self.type_data().map_or(0, |data| data.cargo_size)
	}
	pub fn sight_range(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.sight_range)
	}
	pub fn armor(&self) -> i32 {
		self.type_data().map_or(0, |data| data.armor)
	}
	pub fn towards_facing(&self, offset: f32) -> Point2 {
		self.position
			.offset(offset * self.facing.cos(), offset * self.facing.sin())
	}
	pub fn is_visible(&self) -> bool {
		self.display_type.is_visible()
	}
	pub fn is_snapshot(&self) -> bool {
		self.display_type.is_snapshot()
	}
	pub fn is_hidden(&self) -> bool {
		self.display_type.is_hidden()
	}
	pub fn is_placeholder(&self) -> bool {
		self.display_type.is_placeholder()
	}
	pub fn is_mine(&self) -> bool {
		self.alliance.is_mine()
	}
	pub fn is_enemy(&self) -> bool {
		self.alliance.is_enemy()
	}
	pub fn is_neutral(&self) -> bool {
		self.alliance.is_neutral()
	}
	pub fn is_ally(&self) -> bool {
		self.alliance.is_ally()
	}
	pub fn is_cloaked(&self) -> bool {
		matches!(
			self.cloak,
			CloakState::Cloaked | CloakState::CloakedDetected | CloakState::CloakedAllied
		)
	}
	pub fn is_revealed(&self) -> bool {
		matches!(self.cloak, CloakState::CloakedDetected)
	}
	pub fn can_be_attacked(&self) -> bool {
		matches!(self.cloak, CloakState::NotCloaked | CloakState::CloakedDetected)
	}
	pub fn supply_cost(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.food_required)
	}
	pub fn hits(&self) -> Option<f32> {
		match (self.health, self.shield) {
			(Some(health), Some(shield)) => Some(health + shield),
			(Some(health), None) => Some(health),
			(None, Some(shield)) => Some(shield),
			(None, None) => None,
		}
	}
	pub fn hits_max(&self) -> Option<f32> {
		match (self.health_max, self.shield_max) {
			(Some(health), Some(shield)) => Some(health + shield),
			(Some(health), None) => Some(health),
			(None, Some(shield)) => Some(shield),
			(None, None) => None,
		}
	}
	pub fn speed(&self) -> f32 {
		self.type_data().map_or(0.0, |data| data.movement_speed)
	}
	pub fn real_speed(&self) -> f32 {
		self.calculate_speed(None)
	}
	pub fn calculate_speed(&self, upgrades: Option<&Vec<UpgradeId>>) -> f32 {
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
		if let Some(upgrades) = upgrades.or_else(|| {
			if self.is_mine() {
				Some(&self.data.upgrades)
			} else {
				None
			}
		}) {
			if let Some((upgrade_id, increase)) = SPEED_UPGRADES.get(&unit_type) {
				if upgrades.contains(upgrade_id) {
					speed *= increase;
				}
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
		else if let Some(upgrades) = upgrades {
			if !upgrades.is_empty() {
				if let Some((upgrade_id, increase)) = OFF_CREEP_SPEED_UPGRADES.get(&unit_type) {
					if upgrades.contains(upgrade_id) {
						speed *= increase;
					}
				}
			}
		}
		speed
	}
	// Distance unit can travel per one "on_step" iteration
	pub fn distance_per_step(&self) -> f32 {
		self.real_speed() / FRAMES_PER_SECOND * self.data.game_step as f32
	}
	// Distance unit can travel until weapons be ready to fire
	pub fn distance_to_weapon_ready(&self) -> f32 {
		self.real_speed() / FRAMES_PER_SECOND * self.weapon_cooldown.unwrap_or(0.0)
	}
	pub fn attributes(&self) -> Option<&Vec<Attribute>> {
		self.type_data().map(|data| &data.attributes)
	}
	pub fn has_attribute(&self, attribute: Attribute) -> bool {
		self.type_data()
			.map_or(false, |data| data.attributes.contains(&attribute))
	}
	pub fn is_light(&self) -> bool {
		self.has_attribute(Attribute::Light)
	}
	pub fn is_armored(&self) -> bool {
		self.has_attribute(Attribute::Armored)
	}
	pub fn is_biological(&self) -> bool {
		self.has_attribute(Attribute::Biological)
	}
	pub fn is_mechanical(&self) -> bool {
		self.has_attribute(Attribute::Mechanical)
	}
	pub fn is_robotic(&self) -> bool {
		self.has_attribute(Attribute::Robotic)
	}
	pub fn is_psionic(&self) -> bool {
		self.has_attribute(Attribute::Psionic)
	}
	pub fn is_massive(&self) -> bool {
		self.has_attribute(Attribute::Massive)
	}
	pub fn is_structure(&self) -> bool {
		self.has_attribute(Attribute::Structure)
	}
	pub fn is_hover(&self) -> bool {
		self.has_attribute(Attribute::Hover)
	}
	pub fn is_heroic(&self) -> bool {
		self.has_attribute(Attribute::Heroic)
	}
	pub fn is_summoned(&self) -> bool {
		self.has_attribute(Attribute::Summoned)
	}
	pub fn has_buff(&self, buff: BuffId) -> bool {
		self.buffs.contains(&buff)
	}
	pub fn has_any_buff<B: Iterator<Item = BuffId>>(&self, mut buffs: B) -> bool {
		buffs.any(|b| self.buffs.contains(&b))
	}
	pub fn is_carrying_minerals(&self) -> bool {
		self.has_any_buff(
			[
				BuffId::CarryMineralFieldMinerals,
				BuffId::CarryHighYieldMineralFieldMinerals,
			]
			.iter()
			.copied(),
		)
	}
	pub fn is_carrying_vespene(&self) -> bool {
		self.has_any_buff(
			[
				BuffId::CarryHarvestableVespeneGeyserGas,
				BuffId::CarryHarvestableVespeneGeyserGasProtoss,
				BuffId::CarryHarvestableVespeneGeyserGasZerg,
			]
			.iter()
			.copied(),
		)
	}
	pub fn is_carrying_resource(&self) -> bool {
		self.is_carrying_minerals() || self.is_carrying_vespene()
	}
	#[inline]
	pub fn distance<P: Into<Point2>>(&self, other: P) -> f32 {
		self.position.distance(other)
	}
	#[inline]
	pub fn distance_squared<P: Into<Point2>>(&self, other: P) -> f32 {
		self.position.distance_squared(other)
	}
	#[inline]
	pub fn is_closer<P: Into<Point2>>(&self, distance: f32, other: P) -> bool {
		self.distance_squared(other) < distance * distance
	}
	#[inline]
	pub fn is_further<P: Into<Point2>>(&self, distance: f32, other: P) -> bool {
		self.distance_squared(other) > distance * distance
	}
	#[inline]
	fn weapons(&self) -> Option<Vec<Weapon>> {
		self.type_data()
			.map(|data| data.weapons.clone())
			.or_else(|| match self.type_id {
				UnitTypeId::BanelingBurrowed | UnitTypeId::BanelingCocoon => {
					MISSED_WEAPONS.get(&UnitTypeId::Baneling).cloned()
				}
				UnitTypeId::RavagerCocoon => self
					.data
					.game_data
					.units
					.get(&UnitTypeId::Ravager)
					.map(|data| data.weapons.clone()),
				unit_type => MISSED_WEAPONS.get(&unit_type).cloned(),
			})
	}
	pub fn weapon_target(&self) -> Option<TargetType> {
		self.weapons().and_then(|weapons| {
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
		})
	}
	#[rustfmt::skip::macros(matches)]
	pub fn can_attack(&self) -> bool {
		self.weapons().map_or(false, |weapons| !weapons.is_empty())
	}
	#[rustfmt::skip::macros(matches)]
	pub fn can_attack_both(&self) -> bool {
		self.weapons().map_or(false, |weapons| {
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
		})
	}
	#[rustfmt::skip::macros(matches)]
	pub fn can_attack_ground(&self) -> bool {
		self.weapons().map_or(false, |weapons| {
			weapons.iter().any(|w| TARGET_GROUND.contains(&w.target))
		})
	}
	#[rustfmt::skip::macros(matches)]
	pub fn can_attack_air(&self) -> bool {
		self.weapons().map_or(false, |weapons| {
			weapons.iter().any(|w| TARGET_AIR.contains(&w.target))
		})
	}
	pub fn on_cooldown(&self) -> bool {
		self.weapon_cooldown
			.map_or_else(|| panic!("Can't get cooldown on enemies"), |cool| cool > 0.0)
	}
	pub fn max_cooldown(&self) -> Option<f32> {
		self.data.max_cooldowns.borrow().get(&self.type_id).copied()
	}
	pub fn ground_range(&self) -> f32 {
		self.weapons().map_or(0.0, |weapons| {
			weapons
				.iter()
				.find(|w| TARGET_GROUND.contains(&w.target))
				.map_or(0.0, |w| w.range)
		})
	}
	pub fn air_range(&self) -> f32 {
		self.weapons().map_or(0.0, |weapons| {
			weapons
				.iter()
				.find(|w| TARGET_AIR.contains(&w.target))
				.map_or(0.0, |w| w.range)
		})
	}
	pub fn ground_dps(&self) -> f32 {
		self.weapons().map_or(0.0, |weapons| {
			weapons
				.iter()
				.find(|w| TARGET_GROUND.contains(&w.target))
				.map_or(0.0, |w| w.damage * (w.attacks as f32) / w.speed)
		})
	}
	pub fn air_dps(&self) -> f32 {
		self.weapons().map_or(0.0, |weapons| {
			weapons
				.iter()
				.find(|w| TARGET_AIR.contains(&w.target))
				.map_or(0.0, |w| w.damage * (w.attacks as f32) / w.speed)
		})
	}

	// Returns (dps, range)
	pub fn real_weapon_stats(&self) -> (f32, f32) {
		let (damage, speed, range) =
			self.calculate_weapon_stats(CalcTarget::Abstract(TargetType::Any, None), None);
		(damage / speed, range)
	}

	pub fn calculate_weapon_abstract(
		&self,
		target_type: TargetType,
		attributes: Option<&Vec<Attribute>>,
		upgrades: Option<&Vec<UpgradeId>>,
	) -> (f32, f32) {
		let (damage, speed, range) =
			self.calculate_weapon_stats(CalcTarget::Abstract(target_type, attributes), upgrades);
		(damage / speed, range)
	}
	pub fn calculate_weapon_vs(&self, target: &Unit, upgrades: Option<&Vec<UpgradeId>>) -> (f32, f32) {
		let (damage, speed, range) = self.calculate_weapon_stats(CalcTarget::Unit(target), upgrades);
		(damage / speed, range)
	}

	// Returns (damage, cooldown, range)
	#[allow(clippy::mut_range_bound)]
	pub fn calculate_weapon_stats(
		&self,
		target: CalcTarget,
		upgrades: Option<&Vec<UpgradeId>>,
	) -> (f32, f32, f32) {
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
			if self.is_mine() {
				(Some(&*self.data.upgrades), upgrades)
			} else {
				(upgrades, Some(&*self.data.upgrades))
			}
		};

		let (target_type, attributes, target_unit) = match target {
			CalcTarget::Unit(target) => {
				let mut enemy_armor = target.armor() + target.armor_upgrade_level;
				let mut enemy_shield_armor = target.shield_upgrade_level;

				let mut target_has_guardian_shield = false;

				target.buffs.iter().for_each(|buff| match buff {
					BuffId::GuardianShield => target_has_guardian_shield = true,
					BuffId::RavenShredderMissileTint => {
						enemy_armor -= 3;
						enemy_shield_armor -= 3;
					}
					_ => {}
				});

				if let Some(target_upgrades) = target_upgrades {
					if !target_upgrades.is_empty() {
						if target.race().is_terran() {
							if target.is_structure()
								&& target_upgrades.contains(&UpgradeId::TerranBuildingArmor)
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
				}

				(
					if matches!(target.type_id, UnitTypeId::Colossus) {
						TargetType::Any
					} else if target.is_flying {
						TargetType::Air
					} else {
						TargetType::Ground
					},
					target.type_data().map(|data| &data.attributes),
					Some((
						target,
						enemy_armor,
						enemy_shield_armor,
						target_has_guardian_shield,
					)),
				)
			}
			CalcTarget::Abstract(target_type, attributes) => (target_type, attributes, None),
		};

		self.weapons().map_or((0.0, 0.0, 0.0), |weapons| {
			let mut speed_modifier = 1.0;
			let mut range_modifier = 0.0;

			self.buffs.iter().for_each(|buff| match buff {
				BuffId::Stimpack | BuffId::StimpackMarauder => speed_modifier /= 1.5,
				BuffId::TimeWarpProduction => speed_modifier *= 2.0,
				_ => {}
			});

			if let Some(upgrades) = upgrades {
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
						UnitTypeId::PlanetaryFortress
						| UnitTypeId::MissileTurret
						| UnitTypeId::AutoTurret => {
							if upgrades.contains(&UpgradeId::HiSecAutoTracking) {
								range_modifier += 1.0;
							}
						}
						_ => {}
					}
				}
			}

			let damage_bonus_per_upgrade = DAMAGE_BONUS_PER_UPGRADE.get(&self.type_id);
			weapons
				.iter()
				.filter_map(|w| {
					if !(w.target.is_any() || target_type.is_any()) && w.target != target_type {
						return None;
					}

					let damage_bonus_per_upgrade =
						damage_bonus_per_upgrade.and_then(|bonus| bonus.get(&w.target));

					let mut damage = w.damage
						+ (self.attack_upgrade_level
							* damage_bonus_per_upgrade.and_then(|bonus| bonus.0).unwrap_or(1)) as f32;
					let speed = w.speed * speed_modifier;
					let range = w.range + range_modifier;

					// Bonus damage
					if let Some(bonus) = w
						.damage_bonus
						.iter()
						.filter_map(|(attribute, bonus)| {
							if attributes
								.as_ref()
								.map_or(false, |attributes| attributes.contains(attribute))
							{
								let mut damage_bonus_per_upgrade = damage_bonus_per_upgrade
									.and_then(|bonus| bonus.1.get(attribute))
									.copied()
									.unwrap_or(0);

								if let Attribute::Light = attribute {
									if let Some(upgrades) = upgrades {
										if upgrades.contains(&UpgradeId::HighCapacityBarrels) {
											match self.type_id {
												UnitTypeId::Hellion => damage_bonus_per_upgrade += 5,
												UnitTypeId::HellionTank => damage_bonus_per_upgrade += 12,
												_ => {}
											}
										}
									}
								}

								let mut bonus_damage =
									bonus + (self.attack_upgrade_level * damage_bonus_per_upgrade) as f32;

								if let Attribute::Armored = attribute {
									if self.has_buff(BuffId::VoidRaySwarmDamageBoost) {
										bonus_damage += 6.0;
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

					// Substract damage
					match target_unit {
						Some((target, enemy_armor, enemy_shield_armor, target_has_guardian_shield)) => {
							let mut attacks = w.attacks;
							let mut shield_damage = 0.0;
							let mut health_damage = 0.0;

							if let Some(enemy_shield) = target.shield.filter(|shield| shield > &0.0) {
								let enemy_shield_armor = if target_has_guardian_shield && range >= 2.0 {
									(enemy_shield_armor + 2) as f32
								} else {
									enemy_shield_armor as f32
								};
								for _ in 0..attacks {
									if shield_damage >= enemy_shield {
										health_damage = shield_damage - enemy_shield;
										break;
									}
									shield_damage += 0.5_f32.max(damage - enemy_shield_armor);
									attacks -= 1;
								}
							}

							if let Some(enemy_health) = target.health.filter(|health| health > &0.0) {
								let enemy_armor = if target_has_guardian_shield && range >= 2.0 {
									(enemy_armor + 2) as f32
								} else {
									enemy_armor as f32
								};
								for _ in 0..attacks {
									if health_damage >= enemy_health {
										break;
									}
									health_damage += 0.5_f32.max(damage - enemy_armor);
								}
							}

							Some((shield_damage + health_damage, speed, range))
						}
						None => Some((damage * (w.attacks as f32), speed, range)),
					}
				})
				.max_by(|(damage1, ..), (damage2, ..)| damage1.partial_cmp(damage2).unwrap())
				.unwrap_or((0.0, 0.0, 0.0))
		})
	}
	pub fn in_range(&self, target: &Unit, gap: f32) -> bool {
		let range = {
			if !target.is_flying {
				if self.can_attack_ground() {
					self.ground_range()
				} else {
					return false;
				}
			} else if self.can_attack_air() {
				self.air_range()
			} else {
				return false;
			}
		};
		let total_range = (self.radius + target.radius + range + gap).powi(2);
		let distance = self.distance_squared(target);

		// Takes into account that Sieged Tank has a minimum range of 2
		distance <= total_range && (self.type_id != UnitTypeId::SiegeTankSieged || distance > 4.0)
	}
	pub fn in_range_of(&self, threat: &Unit, gap: f32) -> bool {
		threat.in_range(self, gap)
	}
	pub fn damage_bonus(&self) -> Option<(Attribute, f32)> {
		self.weapons().and_then(|weapons| {
			weapons
				.iter()
				.find(|w| !w.damage_bonus.is_empty())
				.map(|w| w.damage_bonus[0])
		})
	}
	pub fn target(&self) -> Target {
		if self.is_idle() {
			Target::None
		} else {
			self.orders[0].target
		}
	}
	pub fn target_pos(&self) -> Option<Point2> {
		match self.target() {
			Target::Pos(pos) => Some(pos),
			_ => None,
		}
	}
	pub fn target_tag(&self) -> Option<u64> {
		match self.target() {
			Target::Tag(tag) => Some(tag),
			_ => None,
		}
	}
	pub fn ordered_ability(&self) -> Option<AbilityId> {
		if self.is_idle() {
			None
		} else {
			Some(self.orders[0].ability)
		}
	}
	pub fn is_idle(&self) -> bool {
		self.orders.is_empty()
	}
	pub fn is_almost_idle(&self) -> bool {
		self.is_idle() || self.orders[0].progress >= 0.95
	}
	pub fn is_unused(&self) -> bool {
		if self.has_reactor() {
			self.orders.len() < 2
		} else {
			self.is_idle()
		}
	}
	pub fn is_almost_unused(&self) -> bool {
		if self.has_reactor() {
			self.orders.len() < 2
				|| (self.orders.len() < 4 && self.orders.iter().take(2).any(|order| order.progress >= 0.95))
		} else {
			self.is_almost_idle()
		}
	}
	pub fn is_using(&self, ability: AbilityId) -> bool {
		!self.is_idle() && self.orders[0].ability == ability
	}
	pub fn is_using_any<A: Iterator<Item = AbilityId>>(&self, mut abilities: A) -> bool {
		!self.is_idle() && abilities.any(|a| self.orders[0].ability == a)
	}
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
	pub fn is_moving(&self) -> bool {
		self.is_using(AbilityId::MoveMove)
	}
	pub fn is_patrolling(&self) -> bool {
		self.is_using(AbilityId::Patrol)
	}
	pub fn is_repairing(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::EffectRepair | AbilityId::EffectRepairSCV | AbilityId::EffectRepairMule
			)
	}
	pub fn is_gathering(&self) -> bool {
		self.is_using(AbilityId::HarvestGather)
	}
	pub fn is_returning(&self) -> bool {
		self.is_using(AbilityId::HarvestReturn)
	}
	pub fn is_collecting(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				AbilityId::HarvestGather | AbilityId::HarvestReturn
			)
	}
	pub fn is_constructing(&self) -> bool {
		!self.is_idle()
			&& matches!(
				self.orders[0].ability,
				// Terran
				AbilityId::TerranBuildCommandCenter
				| AbilityId::TerranBuildSupplyDepot
				| AbilityId::TerranBuildRefinery
				| AbilityId::TerranBuildBarracks
				| AbilityId::TerranBuildEngineeringBay
				| AbilityId::TerranBuildMissileTurret
				| AbilityId::TerranBuildBunker
				| AbilityId::TerranBuildSensorTower
				| AbilityId::TerranBuildGhostAcademy
				| AbilityId::TerranBuildFactory
				| AbilityId::TerranBuildStarport
				| AbilityId::TerranBuildArmory
				| AbilityId::TerranBuildFusionCore
				// Protoss
				| AbilityId::ProtossBuildNexus
				| AbilityId::ProtossBuildPylon
				| AbilityId::ProtossBuildAssimilator
				| AbilityId::ProtossBuildGateway
				| AbilityId::ProtossBuildForge
				| AbilityId::ProtossBuildFleetBeacon
				| AbilityId::ProtossBuildTwilightCouncil
				| AbilityId::ProtossBuildPhotonCannon
				| AbilityId::ProtossBuildStargate
				| AbilityId::ProtossBuildTemplarArchive
				| AbilityId::ProtossBuildDarkShrine
				| AbilityId::ProtossBuildRoboticsBay
				| AbilityId::ProtossBuildRoboticsFacility
				| AbilityId::ProtossBuildCyberneticsCore
				| AbilityId::BuildShieldBattery
				// Zerg
				| AbilityId::ZergBuildHatchery
				| AbilityId::ZergBuildCreepTumor
				| AbilityId::ZergBuildExtractor
				| AbilityId::ZergBuildSpawningPool
				| AbilityId::ZergBuildEvolutionChamber
				| AbilityId::ZergBuildHydraliskDen
				| AbilityId::ZergBuildSpire
				| AbilityId::ZergBuildUltraliskCavern
				| AbilityId::ZergBuildInfestationPit
				| AbilityId::ZergBuildNydusNetwork
				| AbilityId::ZergBuildBanelingNest
				| AbilityId::BuildLurkerDen
				| AbilityId::ZergBuildRoachWarren
				| AbilityId::ZergBuildSpineCrawler
				| AbilityId::ZergBuildSporeCrawler
			)
	}
	// Actions
	pub fn command(&self, ability: AbilityId, target: Target, queue: bool) {
		if !queue && !self.allow_spam && !self.is_idle() {
			let last_order = &self.orders[0];
			if ability == last_order.ability && target == last_order.target {
				return;
			}
		}
		self.data
			.commander
			.borrow_mut()
			.command((self.tag, (ability, target, queue)));
	}
	pub fn use_ability(&self, ability: AbilityId, queue: bool) {
		self.command(ability, Target::None, queue)
	}
	pub fn smart(&self, target: Target, queue: bool) {
		self.command(AbilityId::Smart, target, queue)
	}
	pub fn attack(&self, target: Target, queue: bool) {
		self.command(AbilityId::Attack, target, queue)
	}
	pub fn move_to(&self, target: Target, queue: bool) {
		self.command(AbilityId::MoveMove, target, queue)
	}
	pub fn hold_position(&self, queue: bool) {
		self.command(AbilityId::HoldPosition, Target::None, queue)
	}
	pub fn gather(&self, target: u64, queue: bool) {
		self.command(AbilityId::HarvestGather, Target::Tag(target), queue)
	}
	pub fn return_resource(&self, queue: bool) {
		self.command(AbilityId::HarvestReturn, Target::None, queue)
	}
	pub fn stop(&self, queue: bool) {
		self.command(AbilityId::Stop, Target::None, queue)
	}
	pub fn patrol(&self, target: Target, queue: bool) {
		self.command(AbilityId::Patrol, target, queue)
	}
	pub fn repair(&self, target: u64, queue: bool) {
		self.command(AbilityId::EffectRepair, Target::Tag(target), queue)
	}
	pub fn cancel_building(&self, queue: bool) {
		self.command(AbilityId::CancelBuildInProgress, Target::None, queue)
	}
	pub fn cancel_queue(&self, queue: bool) {
		self.command(AbilityId::CancelQueue5, Target::None, queue)
	}
	pub fn build_gas(&self, target: u64, queue: bool) {
		self.command(
			self.data.game_data.units[&self.data.race_values.gas]
				.ability
				.unwrap(),
			Target::Tag(target),
			queue,
		)
	}
	pub fn build(&self, unit: UnitTypeId, target: Point2, queue: bool) {
		if let Some(type_data) = self.data.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				self.command(ability, Target::Pos(target), queue);
			}
		}
	}
	pub fn train(&self, unit: UnitTypeId, queue: bool) {
		if let Some(type_data) = self.data.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				self.command(ability, Target::None, queue);
			}
		}
	}
	pub fn research(&self, upgrade: UpgradeId, queue: bool) {
		if let Some(type_data) = self.data.game_data.upgrades.get(&upgrade) {
			self.command(type_data.ability, Target::None, queue);
		}
	}
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

impl FromProtoData<ProtoUnit> for Unit {
	fn from_proto_data(data: Rc<DataForUnit>, u: ProtoUnit) -> Self {
		let pos = u.get_pos();
		Self {
			data,
			allow_spam: false,
			display_type: DisplayType::from_proto(u.get_display_type()),
			alliance: Alliance::from_proto(u.get_alliance()),
			tag: u.get_tag(),
			type_id: UnitTypeId::from_u32(u.get_unit_type()).unwrap(),
			owner: u.get_owner() as u32,
			position: Point2::from_proto(pos.clone()),
			position3d: Point3::from_proto(pos.clone()),
			facing: u.get_facing(),
			radius: u.get_radius(),
			build_progress: u.get_build_progress(),
			cloak: CloakState::from_proto(u.get_cloak()),
			buffs: u
				.get_buff_ids()
				.iter()
				.map(|b| BuffId::from_u32(*b).unwrap())
				.collect(),
			detect_range: u.get_detect_range(),
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
			health: u.health,
			health_max: u.health_max,
			shield: u.shield,
			shield_max: u.shield_max,
			energy: u.energy,
			energy_max: u.energy_max,
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
							Target::Pos(Point2::from_proto(pos.clone()))
						}
						Some(ProtoTarget::target_unit_tag(tag)) => Target::Tag(*tag),
						None => Target::None,
					},
					progress: order.get_progress(),
				})
				.collect(),
			add_on_tag: u.add_on_tag,
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
					point: Point2::from_proto(t.get_point().clone()),
					tag: t.tag,
				})
				.collect(),
		}
	}
}

#[variant_checkers]
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum DisplayType {
	Visible,     // Fully visible
	Snapshot,    // Dimmed version of unit left behind after entering fog of war
	Hidden,      // Fully hidden
	Placeholder, // Building that hasn't started construction.
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

#[derive(Clone, PartialEq, Eq)]
pub enum CloakState {
	CloakedUnknown, // Under the fog, so unknown whether it's cloaked or not.
	Cloaked,
	CloakedDetected,
	NotCloaked,
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

#[derive(Clone)]
pub struct UnitOrder {
	pub ability: AbilityId,
	pub target: Target,
	pub progress: f32, // Progress of train abilities. Range 0..1
}

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

#[derive(Clone)]
pub struct RallyTarget {
	pub point: Point2,    // Will always be filled.
	pub tag: Option<u64>, // Only if it's targeting a unit.
}
