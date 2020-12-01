use super::{AbilityId, UnitTypeId};

impl UnitTypeId {
	#[inline]
	pub fn is_worker(self) -> bool {
		matches!(self, UnitTypeId::SCV | UnitTypeId::Drone | UnitTypeId::Probe)
	}
	#[rustfmt::skip::macros(matches)]
	#[inline]
	pub fn is_townhall(self) -> bool {
		matches!(
			self,
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
	#[rustfmt::skip::macros(matches)]
	#[inline]
	pub fn is_addon(self) -> bool {
		matches!(
			self,
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
	#[rustfmt::skip::macros(matches)]
	#[inline]
	pub fn is_melee(self) -> bool {
		matches!(
			self,
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
			| UnitTypeId::HellionTank
		)
	}
	#[rustfmt::skip::macros(matches)]
	#[inline]
	pub fn is_structure(self) -> bool {
		matches!(
			self,
			UnitTypeId::CommandCenter
			| UnitTypeId::CommandCenterFlying
			| UnitTypeId::PlanetaryFortress
			| UnitTypeId::OrbitalCommand
			| UnitTypeId::OrbitalCommandFlying
			| UnitTypeId::SupplyDepot
			| UnitTypeId::SupplyDepotLowered
			| UnitTypeId::SupplyDepotDrop
			| UnitTypeId::Refinery
			| UnitTypeId::Barracks
			| UnitTypeId::BarracksFlying
			| UnitTypeId::EngineeringBay
			| UnitTypeId::Bunker
			| UnitTypeId::SensorTower
			| UnitTypeId::MissileTurret
			| UnitTypeId::Factory
			| UnitTypeId::FactoryFlying
			| UnitTypeId::GhostAcademy
			| UnitTypeId::Starport
			| UnitTypeId::StarportFlying
			| UnitTypeId::Armory
			| UnitTypeId::FusionCore
			| UnitTypeId::TechLab
			| UnitTypeId::BarracksTechLab
			| UnitTypeId::FactoryTechLab
			| UnitTypeId::StarportTechLab
			| UnitTypeId::Reactor
			| UnitTypeId::BarracksReactor
			| UnitTypeId::FactoryReactor
			| UnitTypeId::StarportReactor
			| UnitTypeId::Hatchery
			| UnitTypeId::SpineCrawler
			| UnitTypeId::SporeCrawler
			| UnitTypeId::Extractor
			| UnitTypeId::SpawningPool
			| UnitTypeId::EvolutionChamber
			| UnitTypeId::RoachWarren
			| UnitTypeId::BanelingNest
			| UnitTypeId::CreepTumor
			| UnitTypeId::CreepTumorBurrowed
			| UnitTypeId::CreepTumorQueen
			| UnitTypeId::CreepTumorMissile
			| UnitTypeId::Lair
			| UnitTypeId::HydraliskDen
			| UnitTypeId::LurkerDenMP
			| UnitTypeId::InfestationPit
			| UnitTypeId::Spire
			| UnitTypeId::NydusNetwork
			| UnitTypeId::Hive
			| UnitTypeId::GreaterSpire
			| UnitTypeId::UltraliskCavern
			| UnitTypeId::Nexus
			| UnitTypeId::Pylon
			| UnitTypeId::Assimilator
			| UnitTypeId::Gateway
			| UnitTypeId::Forge
			| UnitTypeId::CyberneticsCore
			| UnitTypeId::PhotonCannon
			| UnitTypeId::ShieldBattery
			| UnitTypeId::RoboticsFacility
			| UnitTypeId::WarpGate
			| UnitTypeId::Stargate
			| UnitTypeId::TwilightCouncil
			| UnitTypeId::RoboticsBay
			| UnitTypeId::FleetBeacon
			| UnitTypeId::TemplarArchive
			| UnitTypeId::DarkShrine
		)
	}
	#[rustfmt::skip::macros(matches)]
	#[inline]
	pub fn is_unit(self) -> bool {
		matches!(
			self,
			UnitTypeId::SCV
			| UnitTypeId::Marine
			| UnitTypeId::Marauder
			| UnitTypeId::Reaper
			| UnitTypeId::Ghost
			| UnitTypeId::Hellion
			| UnitTypeId::HellionTank
			| UnitTypeId::SiegeTank
			| UnitTypeId::SiegeTankSieged
			| UnitTypeId::Cyclone
			| UnitTypeId::WidowMine
			| UnitTypeId::WidowMineBurrowed
			| UnitTypeId::Thor
			| UnitTypeId::ThorAP
			| UnitTypeId::VikingFighter
			| UnitTypeId::VikingAssault
			| UnitTypeId::Medivac
			| UnitTypeId::Liberator
			| UnitTypeId::LiberatorAG
			| UnitTypeId::Raven
			| UnitTypeId::Banshee
			| UnitTypeId::Battlecruiser
			| UnitTypeId::Larva
			| UnitTypeId::Egg
			| UnitTypeId::Drone
			| UnitTypeId::DroneBurrowed
			| UnitTypeId::Queen
			| UnitTypeId::QueenBurrowed
			| UnitTypeId::Zergling
			| UnitTypeId::ZerglingBurrowed
			| UnitTypeId::BanelingCocoon
			| UnitTypeId::Baneling
			| UnitTypeId::BanelingBurrowed
			| UnitTypeId::Roach
			| UnitTypeId::RoachBurrowed
			| UnitTypeId::RavagerCocoon
			| UnitTypeId::Ravager
			| UnitTypeId::RavagerBurrowed
			| UnitTypeId::Hydralisk
			| UnitTypeId::HydraliskBurrowed
			| UnitTypeId::LurkerMPEgg
			| UnitTypeId::LurkerMP
			| UnitTypeId::LurkerMPBurrowed
			| UnitTypeId::Infestor
			| UnitTypeId::InfestorBurrowed
			| UnitTypeId::SwarmHostMP
			| UnitTypeId::SwarmHostBurrowedMP
			| UnitTypeId::Ultralisk
			| UnitTypeId::UltraliskBurrowed
			| UnitTypeId::LocustMP
			| UnitTypeId::LocustMPFlying
			| UnitTypeId::Broodling
			| UnitTypeId::Changeling
			| UnitTypeId::ChangelingZealot
			| UnitTypeId::ChangelingMarine
			| UnitTypeId::ChangelingMarineShield
			| UnitTypeId::ChangelingZergling
			| UnitTypeId::ChangelingZerglingWings
			| UnitTypeId::InfestorTerran
			| UnitTypeId::InfestorTerranBurrowed
			| UnitTypeId::NydusCanal
			| UnitTypeId::Overlord
			| UnitTypeId::OverlordCocoon
			| UnitTypeId::OverlordTransport
			| UnitTypeId::TransportOverlordCocoon
			| UnitTypeId::Overseer
			| UnitTypeId::OverseerSiegeMode
			| UnitTypeId::Mutalisk
			| UnitTypeId::Corruptor
			| UnitTypeId::BroodLordCocoon
			| UnitTypeId::BroodLord
			| UnitTypeId::Viper
			| UnitTypeId::Probe
			| UnitTypeId::Zealot
			| UnitTypeId::Stalker
			| UnitTypeId::Sentry
			| UnitTypeId::Adept
			| UnitTypeId::AdeptPhaseShift
			| UnitTypeId::HighTemplar
			| UnitTypeId::DarkTemplar
			| UnitTypeId::Immortal
			| UnitTypeId::Colossus
			| UnitTypeId::Disruptor
			| UnitTypeId::Archon
			| UnitTypeId::Observer
			| UnitTypeId::ObserverSiegeMode
			| UnitTypeId::WarpPrism
			| UnitTypeId::WarpPrismPhasing
			| UnitTypeId::Phoenix
			| UnitTypeId::VoidRay
			| UnitTypeId::Oracle
			| UnitTypeId::Carrier
			| UnitTypeId::Interceptor
			| UnitTypeId::Tempest
			| UnitTypeId::Mothership
		)
	}
}

impl AbilityId {
	#[inline]
	pub fn is_constructing(self) -> bool {
		matches!(
			self,
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
}
