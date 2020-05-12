use crate::{game_data::TargetType, ids::*, player::Race};
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

	pub(crate) static ref SPEED_UPGRADES: HashMap<UnitTypeId, (UpgradeId, f32)> = {
		let mut map = hashmap![
			// Terran
			UnitTypeId::Banshee => (UpgradeId::BansheeSpeed, 1.3636),
			// Protoss
			UnitTypeId::Zealot => (UpgradeId::Charge, 1.5),
			UnitTypeId::Observer => (UpgradeId::ObserverGraviticBooster, 2.0),
			UnitTypeId::WarpPrism => (UpgradeId::GraviticDrive, 1.3),
			// Zerg
			UnitTypeId::Overlord => (UpgradeId::Overlordspeed, 2.915),
			UnitTypeId::Overseer => (UpgradeId::Overlordspeed, 1.8015),
			UnitTypeId::Zergling => (UpgradeId::Zerglingmovementspeed, 1.6),
			UnitTypeId::Baneling => (UpgradeId::CentrificalHooks, 1.18),
			UnitTypeId::Roach => (UpgradeId::GlialReconstitution, 1.333_333_4),
			UnitTypeId::LurkerMP => (UpgradeId::DiggingClaws, 1.1),
		];
		if cfg!(windows) {
			map.insert(UnitTypeId::Medivac, (UpgradeId::MedivacRapidDeployment, 1.18));
			map.insert(UnitTypeId::VoidRay, (UpgradeId::VoidRaySpeedUpgrade, 1.328));
		}
		map
	};
	pub(crate) static ref OFF_CREEP_SPEED_UPGRADES: HashMap<UnitTypeId, (UpgradeId, f32)> = hashmap![
		UnitTypeId::Hydralisk => (UpgradeId::EvolveMuscularAugments, 1.25),
		UnitTypeId::Ultralisk => (UpgradeId::AnabolicSynthesis, 1.2),
	];
	pub(crate) static ref SPEED_ON_CREEP: HashMap<UnitTypeId, f32> = hashmap![
		UnitTypeId::Queen => 2.67,
		UnitTypeId::Zergling => 1.3,
		UnitTypeId::Baneling => 1.3,
		UnitTypeId::Roach => 1.3,
		UnitTypeId::Ravager => 1.3,
		UnitTypeId::Hydralisk => 1.3,
		UnitTypeId::LurkerMP => 1.3,
		UnitTypeId::Ultralisk => 1.3,
		UnitTypeId::Infestor => 1.3,
		UnitTypeId::InfestorTerran => 1.3,
		UnitTypeId::SwarmHostMP => 1.3,
		UnitTypeId::LocustMP => 1.4,
		UnitTypeId::SpineCrawler => 2.5,
		UnitTypeId::SporeCrawler => 2.5,
	];
	pub(crate) static ref SPEED_BUFFS: HashMap<BuffId, f32> = {
		let mut map = hashmap![
			BuffId::Stimpack => 1.5,
			BuffId::StimpackMarauder => 1.5,
			BuffId::ChargeUp => if cfg!(windows) { 2.8 } else { 2.2 },
			BuffId::DutchMarauderSlow => 0.5,
			BuffId::TimeWarpProduction => 0.5,
			BuffId::FungalGrowth => 0.25,
			BuffId::InhibitorZoneTemporalField => 0.65,
		];
		if cfg!(windows) {
			map.insert(BuffId::InhibitorZoneFlyingTemporalField, 0.65);
			map.insert(BuffId::AccelerationZoneTemporalField, 1.35);
			map.insert(BuffId::AccelerationZoneFlyingTemporalField, 1.35);
		}
		map
	};
	pub(crate) static ref WARPGATE_ABILITIES: HashMap<UnitTypeId, AbilityId> = hashmap![
		UnitTypeId::Zealot => AbilityId::WarpGateTrainZealot,
		UnitTypeId::Stalker => AbilityId::WarpGateTrainStalker,
		UnitTypeId::HighTemplar => AbilityId::WarpGateTrainHighTemplar,
		UnitTypeId::DarkTemplar => AbilityId::WarpGateTrainDarkTemplar,
		UnitTypeId::Sentry => AbilityId::WarpGateTrainSentry,
		UnitTypeId::Adept => AbilityId::TrainWarpAdept,
	];
}
