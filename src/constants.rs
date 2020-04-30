use crate::{game_data::TargetType, ids::UnitTypeId, player::Race};
use std::collections::HashMap;

pub const TARGET_GROUND: [TargetType; 2] = [TargetType::Ground, TargetType::Any];
pub const TARGET_AIR: [TargetType; 2] = [TargetType::Air, TargetType::Any];

#[derive(Clone)]
pub struct RaceValues {
	pub start_townhall: UnitTypeId,
	pub townhalls: Vec<UnitTypeId>,
	pub gas_building: UnitTypeId,
	pub supply: UnitTypeId,
	pub worker: UnitTypeId,
}
impl Default for RaceValues {
	fn default() -> Self {
		Self {
			start_townhall: UnitTypeId::NotAUnit,
			townhalls: Vec::new(),
			gas_building: UnitTypeId::NotAUnit,
			supply: UnitTypeId::NotAUnit,
			worker: UnitTypeId::NotAUnit,
		}
	}
}

lazy_static! {
	pub static ref RACE_VALUES: HashMap<Race, RaceValues> = hashmap![
		Race::Terran => RaceValues {
			start_townhall: UnitTypeId::CommandCenter,
			townhalls: vec![
				UnitTypeId::CommandCenter,
				UnitTypeId::OrbitalCommand,
				UnitTypeId::PlanetaryFortress,
				UnitTypeId::CommandCenterFlying,
				UnitTypeId::OrbitalCommandFlying,
			],
			gas_building: UnitTypeId::Refinery,
			supply: UnitTypeId::SupplyDepot,
			worker: UnitTypeId::SCV,
		},
		Race::Zerg => RaceValues {
			start_townhall: UnitTypeId::Hatchery,
			townhalls: vec![UnitTypeId::Hatchery, UnitTypeId::Lair, UnitTypeId::Hive],
			gas_building: UnitTypeId::Extractor,
			supply: UnitTypeId::Overlord,
			worker: UnitTypeId::Drone,
		},
		Race::Protoss => RaceValues {
			start_townhall: UnitTypeId::Nexus,
			townhalls: vec![UnitTypeId::Nexus],
			gas_building: UnitTypeId::Assimilator,
			supply: UnitTypeId::Pylon,
			worker: UnitTypeId::Probe,
		},
	];
	pub static ref TECH_REQUIREMENTS: HashMap<UnitTypeId, UnitTypeId> = hashmap![
		// Terran
		UnitTypeId::MissileTurret => UnitTypeId::EngineeringBay,
		UnitTypeId::SensorTower => UnitTypeId::EngineeringBay,
		UnitTypeId::PlanetaryFortress => UnitTypeId::EngineeringBay,
		UnitTypeId::Barracks => UnitTypeId::SupplyDepot,
		UnitTypeId::OrbitalCommand => UnitTypeId::Barracks,
		UnitTypeId::Bunker => UnitTypeId::Barracks,
		UnitTypeId::Ghost => UnitTypeId::GhostAcademy,
		UnitTypeId::GhostAcademy => UnitTypeId::Barracks,
		UnitTypeId::Factory => UnitTypeId::Barracks,
		UnitTypeId::Armory => UnitTypeId::Factory,
		UnitTypeId::HellionTank => UnitTypeId::Armory,
		UnitTypeId::Thor => UnitTypeId::Armory,
		UnitTypeId::Starport => UnitTypeId::Factory,
		UnitTypeId::FusionCore => UnitTypeId::Starport,
		UnitTypeId::Battlecruiser => UnitTypeId::FusionCore,
		// Protoss
		UnitTypeId::PhotonCannon => UnitTypeId::Forge,
		UnitTypeId::CyberneticsCore => UnitTypeId::Gateway,
		UnitTypeId::Sentry => UnitTypeId::CyberneticsCore,
		UnitTypeId::Stalker => UnitTypeId::CyberneticsCore,
		UnitTypeId::Adept => UnitTypeId::CyberneticsCore,
		UnitTypeId::TwilightCouncil => UnitTypeId::CyberneticsCore,
		UnitTypeId::ShieldBattery => UnitTypeId::CyberneticsCore,
		UnitTypeId::TemplarArchive => UnitTypeId::TwilightCouncil,
		UnitTypeId::DarkShrine => UnitTypeId::TwilightCouncil,
		UnitTypeId::HighTemplar => UnitTypeId::TemplarArchive,
		UnitTypeId::DarkTemplar => UnitTypeId::DarkShrine,
		UnitTypeId::Stargate => UnitTypeId::CyberneticsCore,
		UnitTypeId::Tempest => UnitTypeId::FleetBeacon,
		UnitTypeId::Carrier => UnitTypeId::FleetBeacon,
		UnitTypeId::Mothership => UnitTypeId::FleetBeacon,
		UnitTypeId::RoboticsFacility => UnitTypeId::CyberneticsCore,
		UnitTypeId::RoboticsBay => UnitTypeId::RoboticsFacility,
		UnitTypeId::Colossus => UnitTypeId::RoboticsBay,
		UnitTypeId::Disruptor => UnitTypeId::RoboticsBay,
		// Zerg
		UnitTypeId::Zergling => UnitTypeId::SpawningPool,
		UnitTypeId::Queen => UnitTypeId::SpawningPool,
		UnitTypeId::RoachWarren => UnitTypeId::SpawningPool,
		UnitTypeId::BanelingNest => UnitTypeId::SpawningPool,
		UnitTypeId::SpineCrawler => UnitTypeId::SpawningPool,
		UnitTypeId::SporeCrawler => UnitTypeId::SpawningPool,
		UnitTypeId::Roach => UnitTypeId::RoachWarren,
		UnitTypeId::Baneling => UnitTypeId::BanelingNest,
		UnitTypeId::Lair => UnitTypeId::SpawningPool,
		UnitTypeId::Overseer => UnitTypeId::Lair,
		UnitTypeId::OverlordTransport => UnitTypeId::Lair,
		UnitTypeId::InfestationPit => UnitTypeId::Lair,
		UnitTypeId::Infestor => UnitTypeId::InfestationPit,
		UnitTypeId::SwarmHostMP => UnitTypeId::InfestationPit,
		UnitTypeId::HydraliskDen => UnitTypeId::Lair,
		UnitTypeId::Hydralisk => UnitTypeId::HydraliskDen,
		UnitTypeId::LurkerDenMP => UnitTypeId::HydraliskDen,
		UnitTypeId::LurkerMP => UnitTypeId::LurkerDenMP,
		UnitTypeId::Spire => UnitTypeId::Lair,
		UnitTypeId::Mutalisk => UnitTypeId::Spire,
		UnitTypeId::Corruptor => UnitTypeId::Spire,
		UnitTypeId::NydusNetwork => UnitTypeId::Lair,
		UnitTypeId::Hive => UnitTypeId::InfestationPit,
		UnitTypeId::Viper => UnitTypeId::Hive,
		UnitTypeId::UltraliskCavern => UnitTypeId::Hive,
		UnitTypeId::GreaterSpire => UnitTypeId::Hive,
		UnitTypeId::BroodLord => UnitTypeId::GreaterSpire,
	];
}
