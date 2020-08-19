//! Auto generated with `generate_ids.py` script from `stableid.json`
//! ids of units, ablities, upgrades, buffs and effects.
#![allow(missing_docs)]

mod unit_typeid;
mod ability_id;
mod upgrade_id;
mod buff_id;
mod effect_id;

pub use unit_typeid::UnitTypeId;
pub use ability_id::AbilityId;
pub use upgrade_id::UpgradeId;
pub use buff_id::BuffId;
pub use effect_id::EffectId;

mod impls;
