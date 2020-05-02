use crate::{
	action::{Commander, Target},
	constants::{RaceValues, TARGET_AIR, TARGET_GROUND},
	game_data::{Attribute, GameData, TargetType, UnitTypeData, Weapon},
	game_state::Alliance,
	geometry::{Point2, Point3},
	ids::{AbilityId, BuffId, UnitTypeId, UpgradeId},
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
	pub techlab_tags: Rc<Vec<u64>>,
	pub reactor_tags: Rc<Vec<u64>>,
	pub race_values: Rc<RaceValues>,
	pub max_cooldowns: Rc<RefCell<HashMap<UnitTypeId, f32>>>,
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
	pub armor_upgrade_level: u32,
	pub shield_upgrade_level: u32,

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
		matches!(
			self.type_id,
			UnitTypeId::SCV | UnitTypeId::Drone | UnitTypeId::Probe
		)
	}
	pub fn is_townhall(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::CommandCenter
				| UnitTypeId::OrbitalCommand
				| UnitTypeId::PlanetaryFortress
				| UnitTypeId::CommandCenterFlying
				| UnitTypeId::OrbitalCommandFlying
				| UnitTypeId::Hatchery
				| UnitTypeId::Lair
				| UnitTypeId::Hive
				| UnitTypeId::Nexus
		)
	}
	pub fn is_addon(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::TechLab
				| UnitTypeId::Reactor
				| UnitTypeId::BarracksTechLab
				| UnitTypeId::BarracksReactor
				| UnitTypeId::FactoryTechLab
				| UnitTypeId::FactoryReactor
				| UnitTypeId::StarportTechLab
				| UnitTypeId::StarportReactor
		)
	}
	pub fn is_melee(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::SCV
				| UnitTypeId::Drone
				| UnitTypeId::DroneBurrowed
				| UnitTypeId::Probe
				| UnitTypeId::Zergling
				| UnitTypeId::ZerglingBurrowed
				| UnitTypeId::BanelingCocoon
				| UnitTypeId::Baneling
				| UnitTypeId::BanelingBurrowed
				| UnitTypeId::Broodling
				| UnitTypeId::Zealot
				| UnitTypeId::DarkTemplar
				| UnitTypeId::Ultralisk
				| UnitTypeId::UltraliskBurrowed
		)
	}
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
			.map_or(false, |tag| self.data.techlab_tags.contains(&tag))
	}
	pub fn has_reactor(&self) -> bool {
		self.add_on_tag
			.map_or(false, |tag| self.data.reactor_tags.contains(&tag))
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
	pub fn distance(&self, other: &Unit) -> f32 {
		let dx = self.position.x - other.position.x;
		let dy = self.position.y - other.position.y;
		(dx * dx + dy * dy).sqrt()
	}
	pub fn distance_pos(&self, other: Point2) -> f32 {
		let dx = self.position.x - other.x;
		let dy = self.position.y - other.y;
		(dx * dx + dy * dy).sqrt()
	}
	pub fn distance_squared(&self, other: &Unit) -> f32 {
		let dx = self.position.x - other.position.x;
		let dy = self.position.y - other.position.y;
		dx * dx + dy * dy
	}
	pub fn distance_pos_squared(&self, other: Point2) -> f32 {
		let dx = self.position.x - other.x;
		let dy = self.position.y - other.y;
		dx * dx + dy * dy
	}
	pub fn is_closer_pos(&self, distance: f32, pos: Point2) -> bool {
		self.distance_pos_squared(pos) < distance * distance
	}
	pub fn is_closer(&self, distance: f32, other: &Unit) -> bool {
		self.distance_squared(other) < distance * distance
	}
	pub fn is_further_pos(&self, distance: f32, pos: Point2) -> bool {
		self.distance_pos_squared(pos) > distance * distance
	}
	pub fn is_further(&self, distance: f32, other: &Unit) -> bool {
		self.distance_squared(other) > distance * distance
	}
	fn weapons(&self) -> Option<Vec<Weapon>> {
		self.type_data().map(|data| data.weapons.clone())
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
	pub fn can_attack(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::Battlecruiser
				| UnitTypeId::WidowMineBurrowed
				| UnitTypeId::Bunker
				| UnitTypeId::Baneling
				| UnitTypeId::BanelingBurrowed
				| UnitTypeId::Sentry
				| UnitTypeId::VoidRay
				| UnitTypeId::Carrier
				| UnitTypeId::Oracle
		) || self.weapons().map_or(false, |weapons| !weapons.is_empty())
	}
	pub fn can_attack_both(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::Battlecruiser
				| UnitTypeId::WidowMineBurrowed
				| UnitTypeId::Bunker
				| UnitTypeId::Sentry
				| UnitTypeId::VoidRay
				| UnitTypeId::Carrier
		) || self.weapons().map_or(false, |weapons| {
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
	pub fn can_attack_ground(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::Battlecruiser
				| UnitTypeId::WidowMineBurrowed
				| UnitTypeId::Bunker
				| UnitTypeId::Baneling
				| UnitTypeId::BanelingBurrowed
				| UnitTypeId::Sentry
				| UnitTypeId::VoidRay
				| UnitTypeId::Carrier
				| UnitTypeId::Oracle
		) || self.weapons().map_or(false, |weapons| {
			weapons.iter().any(|w| TARGET_GROUND.contains(&w.target))
		})
	}
	pub fn can_attack_air(&self) -> bool {
		matches!(
			self.type_id,
			UnitTypeId::Battlecruiser
				| UnitTypeId::WidowMineBurrowed
				| UnitTypeId::Bunker
				| UnitTypeId::Sentry
				| UnitTypeId::VoidRay
				| UnitTypeId::Carrier
		) || self.weapons().map_or(false, |weapons| {
			weapons.iter().any(|w| TARGET_AIR.contains(&w.target))
		})
	}
	pub fn on_cooldown(&self) -> bool {
		self.weapon_cooldown
			.map_or_else(|| panic!("Can't get cooldown on enemies"), |cool| cool > 0.0)
	}
	// cooldown < 50%
	pub fn on_half_cooldown(&self) -> bool {
		self.weapon_cooldown.map_or_else(
			|| panic!("Can't get cooldown on enemies"),
			|cool| {
				self.max_cooldown()
					.map_or_else(|| !self.on_cooldown(), |max| cool * 2.0 < max)
			},
		)
	}
	pub fn max_cooldown(&self) -> Option<f32> {
		self.data.max_cooldowns.borrow().get(&self.type_id).copied()
	}
	pub fn ground_range(&self) -> f32 {
		match self.type_id {
			UnitTypeId::Battlecruiser => 6.0,
			UnitTypeId::WidowMineBurrowed => 5.0,
			UnitTypeId::Bunker => 6.0,   // Marine range + 1
			UnitTypeId::Baneling => 2.2, // Splash radius
			UnitTypeId::BanelingBurrowed => 2.2,
			UnitTypeId::Sentry => 5.0,
			UnitTypeId::VoidRay => 6.0,
			UnitTypeId::Carrier => 8.0, // Interceptors launch range
			UnitTypeId::Oracle => 4.0,
			_ => self.weapons().map_or(0.0, |weapons| {
				weapons
					.iter()
					.find(|w| TARGET_GROUND.contains(&w.target))
					.map_or(0.0, |w| w.range)
			}),
		}
	}
	pub fn air_range(&self) -> f32 {
		match self.type_id {
			UnitTypeId::Battlecruiser => 6.0,
			UnitTypeId::WidowMineBurrowed => 5.0,
			UnitTypeId::Bunker => 6.0, // Marine range + 1
			UnitTypeId::Sentry => 5.0,
			UnitTypeId::VoidRay => 6.0,
			UnitTypeId::Carrier => 8.0, // Interceptors launch range
			_ => self.weapons().map_or(0.0, |weapons| {
				weapons
					.iter()
					.find(|w| TARGET_AIR.contains(&w.target))
					.map_or(0.0, |w| w.range)
			}),
		}
	}
	pub fn ground_dps(&self) -> f32 {
		match self.type_id {
			UnitTypeId::Battlecruiser => 35.714_287,
			UnitTypeId::WidowMineBurrowed => 125.0, // Damage of single explosion because dps is not relevant
			UnitTypeId::Bunker => 28.103_045,       // Dps of 4 Marines
			UnitTypeId::Baneling => 20.0,           // Damage of single explosion because dps is not relevant
			UnitTypeId::BanelingBurrowed => 20.0,
			UnitTypeId::Sentry => 6.036_217,
			UnitTypeId::VoidRay => 11.904_762,
			UnitTypeId::Carrier => 26.702_27, // Dps of 8 Interceptors
			UnitTypeId::Oracle => 17.564_404,
			_ => self.weapons().map_or(0.0, |weapons| {
				weapons
					.iter()
					.find(|w| TARGET_GROUND.contains(&w.target))
					.map_or(0.0, |w| w.damage * (w.attacks as f32) / w.speed)
			}),
		}
	}
	pub fn air_dps(&self) -> f32 {
		match self.type_id {
			UnitTypeId::Battlecruiser => 22.321_428,
			UnitTypeId::WidowMineBurrowed => 125.0, // Damage of single explosion because dps is not relevant
			UnitTypeId::Bunker => 28.103_045,       // Dps of 4 Marines
			UnitTypeId::Sentry => 6.036_217,
			UnitTypeId::VoidRay => 11.904_762,
			UnitTypeId::Carrier => 26.702_27, // Dps of 8 Interceptors
			_ => self.weapons().map_or(0.0, |weapons| {
				weapons
					.iter()
					.find(|w| TARGET_AIR.contains(&w.target))
					.map_or(0.0, |w| w.damage * (w.attacks as f32) / w.speed)
			}),
		}
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
		let distance = self.radius + target.radius + range + gap;
		self.distance_squared(target) <= distance * distance
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
	pub fn repair(&self, target: Target, queue: bool) {
		self.command(AbilityId::EffectRepair, target, queue)
	}
	pub fn cancel_building(&self, queue: bool) {
		self.command(AbilityId::CancelBuildInProgress, Target::None, queue)
	}
	pub fn cancel_queue(&self, queue: bool) {
		self.command(AbilityId::CancelQueue5, Target::None, queue)
	}
	pub fn build_gas(&self, target: u64, queue: bool) {
		self.command(
			self.data.game_data.units[&self.data.race_values.gas_building]
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
			armor_upgrade_level: u.get_armor_upgrade_level() as u32,
			shield_upgrade_level: u.get_shield_upgrade_level() as u32,
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
