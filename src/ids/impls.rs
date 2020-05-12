use super::UnitTypeId;

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
		)
	}
}
