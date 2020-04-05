use crate::{
	game_data::TargetType,
	ids::{AbilityId, UnitTypeId},
	player::Race,
};
use std::collections::HashMap;

pub const WORKER_IDS: [UnitTypeId; 3] = [UnitTypeId::SCV, UnitTypeId::Drone, UnitTypeId::Probe];
pub const TOWNHALL_IDS: [UnitTypeId; 9] = [
	UnitTypeId::CommandCenter,
	UnitTypeId::OrbitalCommand,
	UnitTypeId::PlanetaryFortress,
	UnitTypeId::CommandCenterFlying,
	UnitTypeId::OrbitalCommandFlying,
	UnitTypeId::Hatchery,
	UnitTypeId::Lair,
	UnitTypeId::Hive,
	UnitTypeId::Nexus,
];
pub const ADDON_IDS: [UnitTypeId; 8] = [
	UnitTypeId::TechLab,
	UnitTypeId::Reactor,
	UnitTypeId::BarracksTechLab,
	UnitTypeId::BarracksReactor,
	UnitTypeId::FactoryTechLab,
	UnitTypeId::FactoryReactor,
	UnitTypeId::StarportTechLab,
	UnitTypeId::StarportReactor,
];
pub const TARGET_GROUND: [TargetType; 2] = [TargetType::Ground, TargetType::Any];
pub const TARGET_AIR: [TargetType; 2] = [TargetType::Air, TargetType::Any];
pub const CONSTRUCTING_IDS: [AbilityId; 43] = [
	// Terran
	AbilityId::TerranBuildCommandCenter,
	AbilityId::TerranBuildSupplyDepot,
	AbilityId::TerranBuildRefinery,
	AbilityId::TerranBuildBarracks,
	AbilityId::TerranBuildEngineeringBay,
	AbilityId::TerranBuildMissileTurret,
	AbilityId::TerranBuildBunker,
	AbilityId::TerranBuildSensorTower,
	AbilityId::TerranBuildGhostAcademy,
	AbilityId::TerranBuildFactory,
	AbilityId::TerranBuildStarport,
	AbilityId::TerranBuildArmory,
	AbilityId::TerranBuildFusionCore,
	// Protoss
	AbilityId::ProtossBuildNexus,
	AbilityId::ProtossBuildPylon,
	AbilityId::ProtossBuildAssimilator,
	AbilityId::ProtossBuildGateway,
	AbilityId::ProtossBuildForge,
	AbilityId::ProtossBuildFleetBeacon,
	AbilityId::ProtossBuildTwilightCouncil,
	AbilityId::ProtossBuildPhotonCannon,
	AbilityId::ProtossBuildStargate,
	AbilityId::ProtossBuildTemplarArchive,
	AbilityId::ProtossBuildDarkShrine,
	AbilityId::ProtossBuildRoboticsBay,
	AbilityId::ProtossBuildRoboticsFacility,
	AbilityId::ProtossBuildCyberneticsCore,
	AbilityId::BuildShieldBattery,
	// Zerg
	AbilityId::ZergBuildHatchery,
	AbilityId::ZergBuildCreepTumor,
	AbilityId::ZergBuildExtractor,
	AbilityId::ZergBuildSpawningPool,
	AbilityId::ZergBuildEvolutionChamber,
	AbilityId::ZergBuildHydraliskDen,
	AbilityId::ZergBuildSpire,
	AbilityId::ZergBuildUltraliskCavern,
	AbilityId::ZergBuildInfestationPit,
	AbilityId::ZergBuildNydusNetwork,
	AbilityId::ZergBuildBanelingNest,
	AbilityId::BuildLurkerDen,
	AbilityId::ZergBuildRoachWarren,
	AbilityId::ZergBuildSpineCrawler,
	AbilityId::ZergBuildSporeCrawler,
];

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
	pub static ref RACE_VALUES: HashMap<Race, RaceValues> = {
		let mut map = HashMap::new();
		map.insert(
			Race::Terran,
			RaceValues {
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
		);
		map.insert(
			Race::Zerg,
			RaceValues {
				start_townhall: UnitTypeId::Hatchery,
				townhalls: vec![UnitTypeId::Hatchery, UnitTypeId::Lair, UnitTypeId::Hive],
				gas_building: UnitTypeId::Extractor,
				supply: UnitTypeId::Overlord,
				worker: UnitTypeId::Drone,
			},
		);
		map.insert(
			Race::Protoss,
			RaceValues {
				start_townhall: UnitTypeId::Nexus,
				townhalls: vec![UnitTypeId::Nexus],
				gas_building: UnitTypeId::Assimilator,
				supply: UnitTypeId::Pylon,
				worker: UnitTypeId::Probe,
			},
		);
		map
	};
}
