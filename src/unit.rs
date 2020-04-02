use crate::{
	action::{Command, Target},
	game_data::{Attribute, GameData, TargetType, UnitTypeData, Weapon},
	game_state::Alliance,
	geometry::{Point2, Point3},
	ids::{AbilityId, BuffId, UnitTypeId},
	player::Race,
	FromProto, FromProtoGameData, TOWNHALL_IDS, WORKER_IDS,
};
use num_traits::FromPrimitive;
use sc2_proto::raw::{
	CloakState as ProtoCloakState, DisplayType as ProtoDisplayType, Unit as ProtoUnit,
	UnitOrder_oneof_target as ProtoTarget,
};
use std::rc::Rc;

#[derive(Clone)]
pub struct Unit {
	game_data: Rc<GameData>,

	// Fields are populated based on type/alliance
	pub display_type: DisplayType,
	pub alliance: Alliance,

	pub tag: u64,
	pub type_id: UnitTypeId,
	pub owner: u32,
	pub position: Point2,
	pub position3d: Point3,
	pub facing: f32,
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
	pub is_flying: OptionBool,
	pub is_burrowed: OptionBool,
	pub is_hallucination: OptionBool,

	// Not populated for enemies
	pub orders: Vec<UnitOrder>,
	pub add_on_tag: Option<u64>,
	pub passengers: Vec<PassengerUnit>,
	pub cargo_space_taken: Option<u32>,
	pub cargo_space_max: Option<u32>,
	pub assigned_harvesters: Option<u32>,
	pub ideal_harvesters: Option<u32>,
	pub weapon_cooldown: Option<f32>,
	pub engaged_target_tag: Option<u64>,
	pub buff_duration_remain: Option<u32>, // How long a buff or unit is still around (eg mule, broodling, chronoboost).
	pub buff_duration_max: Option<u32>, // How long the maximum duration of buff or unit (eg mule, broodling, chronoboost).
	pub rally_targets: Vec<RallyTarget>,
}
impl Unit {
	fn type_data(&self) -> Option<UnitTypeData> {
		self.game_data.units.get(&self.type_id).cloned()
	}
	pub fn is_worker(&self) -> bool {
		WORKER_IDS.contains(&self.type_id)
	}
	pub fn is_townhall(&self) -> bool {
		TOWNHALL_IDS.contains(&self.type_id)
	}
	pub fn is_ready(&self) -> bool {
		(self.build_progress - 1.0).abs() < std::f32::EPSILON
	}
	pub fn race(&self) -> Race {
		match self.type_data() {
			Some(data) => data.race,
			None => Race::Random,
		}
	}
	pub fn is_visible(&self) -> bool {
		self.display_type == DisplayType::Visible
	}
	pub fn is_snapshot(&self) -> bool {
		self.display_type == DisplayType::Snapshot
	}
	pub fn is_hidden(&self) -> bool {
		self.display_type == DisplayType::Hidden
	}
	pub fn is_placeholder(&self) -> bool {
		self.display_type == DisplayType::Placeholder
	}
	pub fn is_mine(&self) -> bool {
		self.alliance == Alliance::Own
	}
	pub fn is_enemy(&self) -> bool {
		self.alliance == Alliance::Enemy
	}
	pub fn is_neutral(&self) -> bool {
		self.alliance == Alliance::Neutral
	}
	pub fn is_ally(&self) -> bool {
		self.alliance == Alliance::Ally
	}
	pub fn supply_cost(&self) -> f32 {
		match self.type_data() {
			Some(data) => data.food_required,
			None => 0.0,
		}
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
		match self.type_data() {
			Some(data) => data.movement_speed,
			None => 0.0,
		}
	}
	pub fn has_attribute(&self, attribute: Attribute) -> bool {
		match self.type_data() {
			Some(data) => data.attributes.contains(&attribute),
			None => false,
		}
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
	fn weapons(&self) -> Option<Vec<Weapon>> {
		if let Some(data) = self.type_data() {
			return Some(data.weapons);
		}
		None
	}
	pub fn can_attack(&self) -> bool {
		match self.weapons() {
			Some(weapons) => !weapons.is_empty(),
			None => false,
		}
	}
	pub fn can_attack_both(&self) -> bool {
		match self.weapons() {
			Some(weapons) => {
				!weapons.is_empty() && weapons.iter().any(|w| w.target == TargetType::Any)
				/*|| {
					let mut ground = false;
					let mut air = false;
					for w in weapons {
						match w.target {
							TargetType::Ground => ground = true,
							TargetType::Air => air = true,
							_ => break,
						}
						if ground && air {
							break;
						}
					}
					ground && air
				})*/
			}
			None => false,
		}
	}
	pub fn can_attack_ground(&self) -> bool {
		match self.weapons() {
			Some(weapons) => {
				!weapons.is_empty()
					&& weapons
						.iter()
						.any(|w| [TargetType::Ground, TargetType::Any].contains(&w.target))
			}
			None => false,
		}
	}
	pub fn can_attack_air(&self) -> bool {
		match self.weapons() {
			Some(weapons) => {
				!weapons.is_empty()
					&& weapons
						.iter()
						.any(|w| [TargetType::Air, TargetType::Any].contains(&w.target))
			}
			None => false,
		}
	}
	pub fn on_cooldown(&self) -> bool {
		match self.weapon_cooldown {
			Some(cool) => cool > 0.0,
			None => panic!("Can't get cooldown on enemies"),
		}
	}
	pub fn ground_range(&self) -> f32 {
		match self.weapons() {
			Some(weapons) => {
				for w in weapons {
					if w.target == TargetType::Ground {
						return w.range;
					}
				}
				0.0
			}
			None => 0.0,
		}
	}
	pub fn air_range(&self) -> f32 {
		match self.weapons() {
			Some(weapons) => {
				for w in weapons {
					if w.target == TargetType::Air {
						return w.range;
					}
				}
				0.0
			}
			None => 0.0,
		}
	}
	pub fn in_range(&self, target: &Unit, gap: f32) -> bool {
		let range = {
			if !target.is_flying.as_bool() {
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
	pub fn target(&self) -> Target {
		if self.is_idle() {
			Target::None
		} else {
			 self.orders[0].target
		}
	}
	pub fn is_idle(&self) -> bool {
		self.orders.is_empty()
	}
	pub fn is_almost_idle(&self) -> bool {
		self.is_idle() || self.orders[0].progress >= 0.95
	}
	pub fn is_using(&self, ability: AbilityId) -> bool {
		!self.is_idle() && self.orders[0].ability == ability
	}
	pub fn is_using_any<A: Iterator<Item = AbilityId>>(&self, mut abilities: A) -> bool {
		!self.is_idle() && abilities.any(|a| self.orders[0].ability == a)
	}
	pub fn is_attacking(&self) -> bool {
		self.is_using(AbilityId::Attack)
	}
	pub fn is_moving(&self) -> bool {
		self.is_using(AbilityId::MoveMove)
	}
	pub fn is_patrolling(&self) -> bool {
		self.is_using(AbilityId::Patrol)
	}
	pub fn is_repairing(&self) -> bool {
		self.is_using(AbilityId::EffectRepair)
	}
	pub fn is_gathering(&self) -> bool {
		self.is_using(AbilityId::HarvestGather)
	}
	pub fn is_returning(&self) -> bool {
		self.is_using(AbilityId::HarvestReturn)
	}
	pub fn is_collecting(&self) -> bool {
		self.is_using_any(
			[AbilityId::HarvestGather, AbilityId::HarvestReturn]
				.iter()
				.copied(),
		)
	}
	// Actions
	pub fn command(&self, ability: AbilityId, target: Target, queue: bool) -> Option<Command> {
		if !self.is_idle() {
			let last_order = &self.orders[0];
			if !queue && ability == last_order.ability && target == last_order.target {
				return None;
			}
		}
		Some((self.tag, (ability, target, queue)))
	}
	pub fn smart(&self, target: Target, queue: bool) -> Option<Command> {
		self.command(AbilityId::Smart, target, queue)
	}
	pub fn attack(&self, target: Target, queue: bool) -> Option<Command> {
		self.command(AbilityId::Attack, target, queue)
	}
	pub fn move_to(&self, target: Target, queue: bool) -> Option<Command> {
		self.command(AbilityId::MoveMove, target, queue)
	}
	pub fn hold_position(&self, queue: bool) -> Option<Command> {
		self.command(AbilityId::HoldPosition, Target::None, queue)
	}
	pub fn gather(&self, target: Target, queue: bool) -> Option<Command> {
		self.command(AbilityId::HarvestGather, target, queue)
	}
	pub fn return_resource(&self, queue: bool) -> Option<Command> {
		self.command(AbilityId::HarvestReturn, Target::None, queue)
	}
	pub fn stop(&self, queue: bool) -> Option<Command> {
		self.command(AbilityId::Stop, Target::None, queue)
	}
	pub fn patrol(&self, target: Target, queue: bool) -> Option<Command> {
		self.command(AbilityId::Patrol, target, queue)
	}
	pub fn repair(&self, target: Target, queue: bool) -> Option<Command> {
		self.command(AbilityId::EffectRepair, target, queue)
	}
	pub fn build(&self, unit: UnitTypeId, target: Target, queue: bool) -> Option<Command> {
		if let Some(type_data) = self.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				return self.command(ability, target, queue);
			}
		}
		None
	}
	pub fn train(&self, unit: UnitTypeId, queue: bool) -> Option<Command> {
		if let Some(type_data) = self.game_data.units.get(&unit) {
			if let Some(ability) = type_data.ability {
				return self.command(ability, Target::None, queue);
			}
		}
		None
	}
}
impl FromProtoGameData<ProtoUnit> for Unit {
	fn from_proto_game_data(game_data: Rc<GameData>, u: ProtoUnit) -> Self {
		let pos = u.get_pos();
		Self {
			game_data,
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
			health: {
				if u.has_health() {
					Some(u.get_health())
				} else {
					None
				}
			},
			health_max: {
				if u.has_health_max() {
					Some(u.get_health_max())
				} else {
					None
				}
			},
			shield: {
				if u.has_shield() {
					Some(u.get_shield())
				} else {
					None
				}
			},
			shield_max: {
				if u.has_shield_max() {
					Some(u.get_shield_max())
				} else {
					None
				}
			},
			energy: {
				if u.has_energy() {
					Some(u.get_energy())
				} else {
					None
				}
			},
			energy_max: {
				if u.has_energy_max() {
					Some(u.get_energy_max())
				} else {
					None
				}
			},
			mineral_contents: {
				if u.has_mineral_contents() {
					Some(u.get_mineral_contents() as u32)
				} else {
					None
				}
			},
			vespene_contents: {
				if u.has_vespene_contents() {
					Some(u.get_vespene_contents() as u32)
				} else {
					None
				}
			},
			is_flying: {
				if u.has_is_flying() {
					OptionBool::from_bool(u.get_is_flying())
				} else {
					OptionBool::Unknown
				}
			},
			is_burrowed: {
				if u.has_is_burrowed() {
					OptionBool::from_bool(u.get_is_burrowed())
				} else {
					OptionBool::Unknown
				}
			},
			is_hallucination: {
				if u.has_is_hallucination() {
					OptionBool::from_bool(u.get_is_hallucination())
				} else {
					OptionBool::Unknown
				}
			},
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
			add_on_tag: {
				if u.has_add_on_tag() {
					Some(u.get_add_on_tag())
				} else {
					None
				}
			},
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
			cargo_space_taken: {
				if u.has_cargo_space_taken() {
					Some(u.get_cargo_space_taken() as u32)
				} else {
					None
				}
			},
			cargo_space_max: {
				if u.has_cargo_space_max() {
					Some(u.get_cargo_space_max() as u32)
				} else {
					None
				}
			},
			assigned_harvesters: {
				if u.has_assigned_harvesters() {
					Some(u.get_assigned_harvesters() as u32)
				} else {
					None
				}
			},
			ideal_harvesters: {
				if u.has_ideal_harvesters() {
					Some(u.get_ideal_harvesters() as u32)
				} else {
					None
				}
			},
			weapon_cooldown: {
				if u.has_weapon_cooldown() {
					Some(u.get_weapon_cooldown())
				} else {
					None
				}
			},
			engaged_target_tag: {
				if u.has_engaged_target_tag() {
					Some(u.get_engaged_target_tag())
				} else {
					None
				}
			},
			buff_duration_remain: {
				if u.has_buff_duration_remain() {
					Some(u.get_buff_duration_remain() as u32)
				} else {
					None
				}
			},
			buff_duration_max: {
				if u.has_buff_duration_max() {
					Some(u.get_buff_duration_max() as u32)
				} else {
					None
				}
			},
			rally_targets: u
				.get_rally_targets()
				.iter()
				.map(|t| RallyTarget {
					point: Point2::from_proto(t.get_point().clone()),
					tag: {
						if t.has_tag() {
							Some(t.get_tag())
						} else {
							None
						}
					},
				})
				.collect(),
		}
	}
}

#[derive(Clone)]
pub enum OptionBool {
	False,
	True,
	Unknown,
}
impl OptionBool {
	/*
	pub fn from_option(opt: Option<bool>) -> Self {
		match opt {
			Some(false) => OptionBool::False,
			Some(true) => OptionBool::True,
			None => OptionBool::Unknown,
		}
	}
	*/
	pub fn from_bool(b: bool) -> Self {
		if b {
			OptionBool::True
		} else {
			OptionBool::False
		}
	}
	pub fn as_bool(&self) -> bool {
		match self {
			OptionBool::False => false,
			OptionBool::True => true,
			OptionBool::Unknown => false,
		}
	}
	pub fn as_bool_maybe(&self) -> bool {
		match self {
			OptionBool::False => false,
			OptionBool::True => true,
			OptionBool::Unknown => true,
		}
	}
}

#[derive(Clone, PartialEq, Eq)]
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
