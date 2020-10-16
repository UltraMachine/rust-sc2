//! Data structures for executing actions and analyzing actions failure.

use crate::{
	geometry::{Point2, Point3},
	ids::AbilityId,
	FromProto, IntoProto,
};
use num_traits::{FromPrimitive, ToPrimitive};
use rustc_hash::FxHashMap;
use sc2_proto::{
	error::ActionResult as ProtoActionResult,
	raw::{ActionRawUnitCommand_oneof_target as ProtoTarget, ActionRaw_oneof_action as ProtoRawAction},
	sc2api::{Action as ProtoAction, ActionChat_Channel, ActionError as ProtoActionError},
};

// pub(crate) type Command = (u64, (AbilityId, Target, bool));

#[derive(Default, Clone)]
pub(crate) struct Commander {
	pub commands: FxHashMap<(AbilityId, Target, bool), Vec<u64>>,
	pub autocast: FxHashMap<AbilityId, Vec<u64>>,
}

/// Target of ability used by unit.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum Target {
	/// Ability target is position (move, build, ...).
	Pos(Point2),
	/// Ability target is unit (attack, repair, heal, ...).
	Tag(u64),
	/// Ability don't require target (train, morph, research, ...).
	None,
}

#[doc(hidden)]
#[derive(Debug, Clone)]
pub enum Action {
	UnitCommand(AbilityId, Target, Vec<u64>, bool),
	CameraMove(Point3),
	ToggleAutocast(AbilityId, Vec<u64>),
	Chat(String, bool),
}
impl IntoProto<ProtoAction> for &Action {
	fn into_proto(self) -> ProtoAction {
		let mut action = ProtoAction::new();
		match self {
			Action::Chat(message, team_only) => {
				let chat_action = action.mut_action_chat();
				chat_action.set_channel({
					if *team_only {
						ActionChat_Channel::Team
					} else {
						ActionChat_Channel::Broadcast
					}
				});
				chat_action.set_message(message.to_string());
			}
			Action::UnitCommand(ability, target, units, queue) => {
				let unit_command = action.mut_action_raw().mut_unit_command();
				unit_command.set_ability_id(ability.to_i32().unwrap());
				match target {
					Target::Pos(pos) => unit_command.set_target_world_space_pos(pos.into_proto()),
					Target::Tag(tag) => unit_command.set_target_unit_tag(*tag),
					Target::None => {}
				}
				unit_command.set_unit_tags(units.to_vec());
				unit_command.set_queue_command(*queue);
			}
			Action::CameraMove(pos) => {
				let camera_move = action.mut_action_raw().mut_camera_move();
				camera_move.set_center_world_space(pos.into_proto());
			}
			Action::ToggleAutocast(ability, units) => {
				let toggle_autocast = action.mut_action_raw().mut_toggle_autocast();
				toggle_autocast.set_ability_id(ability.to_i32().unwrap());
				toggle_autocast.set_unit_tags(units.to_vec());
			}
		}
		action
	}
}
impl FromProto<&ProtoAction> for Option<Action> {
	fn from_proto(action: &ProtoAction) -> Self {
		// let game_loop: u32 = action.get_game_loop();
		if action.has_action_raw() {
			match &action.get_action_raw().action {
				Some(ProtoRawAction::unit_command(unit_command)) => Some(Action::UnitCommand(
					AbilityId::from_i32(unit_command.get_ability_id()).unwrap(),
					{
						match &unit_command.target {
							Some(ProtoTarget::target_world_space_pos(pos)) => {
								Target::Pos(Point2::from_proto(pos))
							}
							Some(ProtoTarget::target_unit_tag(tag)) => Target::Tag(*tag),
							None => Target::None,
						}
					},
					unit_command.get_unit_tags().to_vec(),
					unit_command.get_queue_command(),
				)),
				Some(ProtoRawAction::camera_move(camera_move)) => Some(Action::CameraMove(
					Point3::from_proto(camera_move.get_center_world_space()),
				)),
				Some(ProtoRawAction::toggle_autocast(toggle_autocast)) => Some(Action::ToggleAutocast(
					AbilityId::from_i32(toggle_autocast.get_ability_id()).unwrap(),
					toggle_autocast.get_unit_tags().to_vec(),
				)),
				None => unreachable!(),
			}
		} else if action.has_action_chat() {
			let chat = action.get_action_chat();
			Some(Action::Chat(chat.get_message().to_string(), {
				match chat.get_channel() {
					ActionChat_Channel::Broadcast => false,
					ActionChat_Channel::Team => true,
				}
			}))
		} else {
			None
		}
	}
}

/// Structure used to analyze actions failed on previous game step.
/// Stored in [`state.action_errors`](crate::game_state::GameState::action_errors).
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct ActionError {
	/// Tag of unit that was executing action.
	pub unit: u64,
	/// Ability that was used by this unit.
	pub ability: AbilityId,
	/// Result of executed action.
	pub result: ActionResult,
}
impl FromProto<&ProtoActionError> for ActionError {
	fn from_proto(e: &ProtoActionError) -> Self {
		Self {
			unit: e.get_unit_tag(),
			ability: AbilityId::from_u64(e.get_ability_id()).unwrap(),
			result: ActionResult::from_proto(e.get_result()),
		}
	}
}

/// Result of executed action.
#[allow(missing_docs)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ActionResult {
	Success,
	NotSupported,
	Error,
	CantQueueThatOrder,
	Retry,
	Cooldown,
	QueueIsFull,
	RallyQueueIsFull,
	NotEnoughMinerals,
	NotEnoughVespene,
	NotEnoughTerrazine,
	NotEnoughCustom,
	NotEnoughFood,
	FoodUsageImpossible,
	NotEnoughLife,
	NotEnoughShields,
	NotEnoughEnergy,
	LifeSuppressed,
	ShieldsSuppressed,
	EnergySuppressed,
	NotEnoughCharges,
	CantAddMoreCharges,
	TooMuchMinerals,
	TooMuchVespene,
	TooMuchTerrazine,
	TooMuchCustom,
	TooMuchFood,
	TooMuchLife,
	TooMuchShields,
	TooMuchEnergy,
	MustTargetUnitWithLife,
	MustTargetUnitWithShields,
	MustTargetUnitWithEnergy,
	CantTrade,
	CantSpend,
	CantTargetThatUnit,
	CouldntAllocateUnit,
	UnitCantMove,
	TransportIsHoldingPosition,
	BuildTechRequirementsNotMet,
	CantFindPlacementLocation,
	CantBuildOnThat,
	CantBuildTooCloseToDropOff,
	CantBuildLocationInvalid,
	CantSeeBuildLocation,
	CantBuildTooCloseToCreepSource,
	CantBuildTooCloseToResources,
	CantBuildTooFarFromWater,
	CantBuildTooFarFromCreepSource,
	CantBuildTooFarFromBuildPowerSource,
	CantBuildOnDenseTerrain,
	CantTrainTooFarFromTrainPowerSource,
	CantLandLocationInvalid,
	CantSeeLandLocation,
	CantLandTooCloseToCreepSource,
	CantLandTooCloseToResources,
	CantLandTooFarFromWater,
	CantLandTooFarFromCreepSource,
	CantLandTooFarFromBuildPowerSource,
	CantLandTooFarFromTrainPowerSource,
	CantLandOnDenseTerrain,
	AddOnTooFarFromBuilding,
	MustBuildRefineryFirst,
	BuildingIsUnderConstruction,
	CantFindDropOff,
	CantLoadOtherPlayersUnits,
	NotEnoughRoomToLoadUnit,
	CantUnloadUnitsThere,
	CantWarpInUnitsThere,
	CantLoadImmobileUnits,
	CantRechargeImmobileUnits,
	CantRechargeUnderConstructionUnits,
	CantLoadThatUnit,
	NoCargoToUnload,
	LoadAllNoTargetsFound,
	NotWhileOccupied,
	CantAttackWithoutAmmo,
	CantHoldAnyMoreAmmo,
	TechRequirementsNotMet,
	MustLockdownUnitFirst,
	MustTargetUnit,
	MustTargetInventory,
	MustTargetVisibleUnit,
	MustTargetVisibleLocation,
	MustTargetWalkableLocation,
	MustTargetPawnableUnit,
	YouCantControlThatUnit,
	YouCantIssueCommandsToThatUnit,
	MustTargetResources,
	RequiresHealTarget,
	RequiresRepairTarget,
	NoItemsToDrop,
	CantHoldAnyMoreItems,
	CantHoldThat,
	TargetHasNoInventory,
	CantDropThisItem,
	CantMoveThisItem,
	CantPawnThisUnit,
	MustTargetCaster,
	CantTargetCaster,
	MustTargetOuter,
	CantTargetOuter,
	MustTargetYourOwnUnits,
	CantTargetYourOwnUnits,
	MustTargetFriendlyUnits,
	CantTargetFriendlyUnits,
	MustTargetNeutralUnits,
	CantTargetNeutralUnits,
	MustTargetEnemyUnits,
	CantTargetEnemyUnits,
	MustTargetAirUnits,
	CantTargetAirUnits,
	MustTargetGroundUnits,
	CantTargetGroundUnits,
	MustTargetStructures,
	CantTargetStructures,
	MustTargetLightUnits,
	CantTargetLightUnits,
	MustTargetArmoredUnits,
	CantTargetArmoredUnits,
	MustTargetBiologicalUnits,
	CantTargetBiologicalUnits,
	MustTargetHeroicUnits,
	CantTargetHeroicUnits,
	MustTargetRoboticUnits,
	CantTargetRoboticUnits,
	MustTargetMechanicalUnits,
	CantTargetMechanicalUnits,
	MustTargetPsionicUnits,
	CantTargetPsionicUnits,
	MustTargetMassiveUnits,
	CantTargetMassiveUnits,
	MustTargetMissile,
	CantTargetMissile,
	MustTargetWorkerUnits,
	CantTargetWorkerUnits,
	MustTargetEnergyCapableUnits,
	CantTargetEnergyCapableUnits,
	MustTargetShieldCapableUnits,
	CantTargetShieldCapableUnits,
	MustTargetFlyers,
	CantTargetFlyers,
	MustTargetBuriedUnits,
	CantTargetBuriedUnits,
	MustTargetCloakedUnits,
	CantTargetCloakedUnits,
	MustTargetUnitsInAStasisField,
	CantTargetUnitsInAStasisField,
	MustTargetUnderConstructionUnits,
	CantTargetUnderConstructionUnits,
	MustTargetDeadUnits,
	CantTargetDeadUnits,
	MustTargetRevivableUnits,
	CantTargetRevivableUnits,
	MustTargetHiddenUnits,
	CantTargetHiddenUnits,
	CantRechargeOtherPlayersUnits,
	MustTargetHallucinations,
	CantTargetHallucinations,
	MustTargetInvulnerableUnits,
	CantTargetInvulnerableUnits,
	MustTargetDetectedUnits,
	CantTargetDetectedUnits,
	CantTargetUnitWithEnergy,
	CantTargetUnitWithShields,
	MustTargetUncommandableUnits,
	CantTargetUncommandableUnits,
	MustTargetPreventDefeatUnits,
	CantTargetPreventDefeatUnits,
	MustTargetPreventRevealUnits,
	CantTargetPreventRevealUnits,
	MustTargetPassiveUnits,
	CantTargetPassiveUnits,
	MustTargetStunnedUnits,
	CantTargetStunnedUnits,
	MustTargetSummonedUnits,
	CantTargetSummonedUnits,
	MustTargetUser1,
	CantTargetUser1,
	MustTargetUnstoppableUnits,
	CantTargetUnstoppableUnits,
	MustTargetResistantUnits,
	CantTargetResistantUnits,
	MustTargetDazedUnits,
	CantTargetDazedUnits,
	CantLockdown,
	CantMindControl,
	MustTargetDestructibles,
	CantTargetDestructibles,
	MustTargetItems,
	CantTargetItems,
	NoCalldownAvailable,
	WaypointListFull,
	MustTargetRace,
	CantTargetRace,
	MustTargetSimilarUnits,
	CantTargetSimilarUnits,
	CantFindEnoughTargets,
	AlreadySpawningLarva,
	CantTargetExhaustedResources,
	CantUseMinimap,
	CantUseInfoPanel,
	OrderQueueIsFull,
	CantHarvestThatResource,
	HarvestersNotRequired,
	AlreadyTargeted,
	CantAttackWeaponsDisabled,
	CouldntReachTarget,
	TargetIsOutOfRange,
	TargetIsTooClose,
	TargetIsOutOfArc,
	CantFindTeleportLocation,
	InvalidItemClass,
	CantFindCancelOrder,
}
impl FromProto<ProtoActionResult> for ActionResult {
	fn from_proto(result: ProtoActionResult) -> Self {
		match result {
			ProtoActionResult::Success => ActionResult::Success,
			ProtoActionResult::NotSupported => ActionResult::NotSupported,
			ProtoActionResult::Error => ActionResult::Error,
			ProtoActionResult::CantQueueThatOrder => ActionResult::CantQueueThatOrder,
			ProtoActionResult::Retry => ActionResult::Retry,
			ProtoActionResult::Cooldown => ActionResult::Cooldown,
			ProtoActionResult::QueueIsFull => ActionResult::QueueIsFull,
			ProtoActionResult::RallyQueueIsFull => ActionResult::RallyQueueIsFull,
			ProtoActionResult::NotEnoughMinerals => ActionResult::NotEnoughMinerals,
			ProtoActionResult::NotEnoughVespene => ActionResult::NotEnoughVespene,
			ProtoActionResult::NotEnoughTerrazine => ActionResult::NotEnoughTerrazine,
			ProtoActionResult::NotEnoughCustom => ActionResult::NotEnoughCustom,
			ProtoActionResult::NotEnoughFood => ActionResult::NotEnoughFood,
			ProtoActionResult::FoodUsageImpossible => ActionResult::FoodUsageImpossible,
			ProtoActionResult::NotEnoughLife => ActionResult::NotEnoughLife,
			ProtoActionResult::NotEnoughShields => ActionResult::NotEnoughShields,
			ProtoActionResult::NotEnoughEnergy => ActionResult::NotEnoughEnergy,
			ProtoActionResult::LifeSuppressed => ActionResult::LifeSuppressed,
			ProtoActionResult::ShieldsSuppressed => ActionResult::ShieldsSuppressed,
			ProtoActionResult::EnergySuppressed => ActionResult::EnergySuppressed,
			ProtoActionResult::NotEnoughCharges => ActionResult::NotEnoughCharges,
			ProtoActionResult::CantAddMoreCharges => ActionResult::CantAddMoreCharges,
			ProtoActionResult::TooMuchMinerals => ActionResult::TooMuchMinerals,
			ProtoActionResult::TooMuchVespene => ActionResult::TooMuchVespene,
			ProtoActionResult::TooMuchTerrazine => ActionResult::TooMuchTerrazine,
			ProtoActionResult::TooMuchCustom => ActionResult::TooMuchCustom,
			ProtoActionResult::TooMuchFood => ActionResult::TooMuchFood,
			ProtoActionResult::TooMuchLife => ActionResult::TooMuchLife,
			ProtoActionResult::TooMuchShields => ActionResult::TooMuchShields,
			ProtoActionResult::TooMuchEnergy => ActionResult::TooMuchEnergy,
			ProtoActionResult::MustTargetUnitWithLife => ActionResult::MustTargetUnitWithLife,
			ProtoActionResult::MustTargetUnitWithShields => ActionResult::MustTargetUnitWithShields,
			ProtoActionResult::MustTargetUnitWithEnergy => ActionResult::MustTargetUnitWithEnergy,
			ProtoActionResult::CantTrade => ActionResult::CantTrade,
			ProtoActionResult::CantSpend => ActionResult::CantSpend,
			ProtoActionResult::CantTargetThatUnit => ActionResult::CantTargetThatUnit,
			ProtoActionResult::CouldntAllocateUnit => ActionResult::CouldntAllocateUnit,
			ProtoActionResult::UnitCantMove => ActionResult::UnitCantMove,
			ProtoActionResult::TransportIsHoldingPosition => ActionResult::TransportIsHoldingPosition,
			ProtoActionResult::BuildTechRequirementsNotMet => ActionResult::BuildTechRequirementsNotMet,
			ProtoActionResult::CantFindPlacementLocation => ActionResult::CantFindPlacementLocation,
			ProtoActionResult::CantBuildOnThat => ActionResult::CantBuildOnThat,
			ProtoActionResult::CantBuildTooCloseToDropOff => ActionResult::CantBuildTooCloseToDropOff,
			ProtoActionResult::CantBuildLocationInvalid => ActionResult::CantBuildLocationInvalid,
			ProtoActionResult::CantSeeBuildLocation => ActionResult::CantSeeBuildLocation,
			ProtoActionResult::CantBuildTooCloseToCreepSource => ActionResult::CantBuildTooCloseToCreepSource,
			ProtoActionResult::CantBuildTooCloseToResources => ActionResult::CantBuildTooCloseToResources,
			ProtoActionResult::CantBuildTooFarFromWater => ActionResult::CantBuildTooFarFromWater,
			ProtoActionResult::CantBuildTooFarFromCreepSource => ActionResult::CantBuildTooFarFromCreepSource,
			ProtoActionResult::CantBuildTooFarFromBuildPowerSource => {
				ActionResult::CantBuildTooFarFromBuildPowerSource
			}
			ProtoActionResult::CantBuildOnDenseTerrain => ActionResult::CantBuildOnDenseTerrain,
			ProtoActionResult::CantTrainTooFarFromTrainPowerSource => {
				ActionResult::CantTrainTooFarFromTrainPowerSource
			}
			ProtoActionResult::CantLandLocationInvalid => ActionResult::CantLandLocationInvalid,
			ProtoActionResult::CantSeeLandLocation => ActionResult::CantSeeLandLocation,
			ProtoActionResult::CantLandTooCloseToCreepSource => ActionResult::CantLandTooCloseToCreepSource,
			ProtoActionResult::CantLandTooCloseToResources => ActionResult::CantLandTooCloseToResources,
			ProtoActionResult::CantLandTooFarFromWater => ActionResult::CantLandTooFarFromWater,
			ProtoActionResult::CantLandTooFarFromCreepSource => ActionResult::CantLandTooFarFromCreepSource,
			ProtoActionResult::CantLandTooFarFromBuildPowerSource => {
				ActionResult::CantLandTooFarFromBuildPowerSource
			}
			ProtoActionResult::CantLandTooFarFromTrainPowerSource => {
				ActionResult::CantLandTooFarFromTrainPowerSource
			}
			ProtoActionResult::CantLandOnDenseTerrain => ActionResult::CantLandOnDenseTerrain,
			ProtoActionResult::AddOnTooFarFromBuilding => ActionResult::AddOnTooFarFromBuilding,
			ProtoActionResult::MustBuildRefineryFirst => ActionResult::MustBuildRefineryFirst,
			ProtoActionResult::BuildingIsUnderConstruction => ActionResult::BuildingIsUnderConstruction,
			ProtoActionResult::CantFindDropOff => ActionResult::CantFindDropOff,
			ProtoActionResult::CantLoadOtherPlayersUnits => ActionResult::CantLoadOtherPlayersUnits,
			ProtoActionResult::NotEnoughRoomToLoadUnit => ActionResult::NotEnoughRoomToLoadUnit,
			ProtoActionResult::CantUnloadUnitsThere => ActionResult::CantUnloadUnitsThere,
			ProtoActionResult::CantWarpInUnitsThere => ActionResult::CantWarpInUnitsThere,
			ProtoActionResult::CantLoadImmobileUnits => ActionResult::CantLoadImmobileUnits,
			ProtoActionResult::CantRechargeImmobileUnits => ActionResult::CantRechargeImmobileUnits,
			ProtoActionResult::CantRechargeUnderConstructionUnits => {
				ActionResult::CantRechargeUnderConstructionUnits
			}
			ProtoActionResult::CantLoadThatUnit => ActionResult::CantLoadThatUnit,
			ProtoActionResult::NoCargoToUnload => ActionResult::NoCargoToUnload,
			ProtoActionResult::LoadAllNoTargetsFound => ActionResult::LoadAllNoTargetsFound,
			ProtoActionResult::NotWhileOccupied => ActionResult::NotWhileOccupied,
			ProtoActionResult::CantAttackWithoutAmmo => ActionResult::CantAttackWithoutAmmo,
			ProtoActionResult::CantHoldAnyMoreAmmo => ActionResult::CantHoldAnyMoreAmmo,
			ProtoActionResult::TechRequirementsNotMet => ActionResult::TechRequirementsNotMet,
			ProtoActionResult::MustLockdownUnitFirst => ActionResult::MustLockdownUnitFirst,
			ProtoActionResult::MustTargetUnit => ActionResult::MustTargetUnit,
			ProtoActionResult::MustTargetInventory => ActionResult::MustTargetInventory,
			ProtoActionResult::MustTargetVisibleUnit => ActionResult::MustTargetVisibleUnit,
			ProtoActionResult::MustTargetVisibleLocation => ActionResult::MustTargetVisibleLocation,
			ProtoActionResult::MustTargetWalkableLocation => ActionResult::MustTargetWalkableLocation,
			ProtoActionResult::MustTargetPawnableUnit => ActionResult::MustTargetPawnableUnit,
			ProtoActionResult::YouCantControlThatUnit => ActionResult::YouCantControlThatUnit,
			ProtoActionResult::YouCantIssueCommandsToThatUnit => ActionResult::YouCantIssueCommandsToThatUnit,
			ProtoActionResult::MustTargetResources => ActionResult::MustTargetResources,
			ProtoActionResult::RequiresHealTarget => ActionResult::RequiresHealTarget,
			ProtoActionResult::RequiresRepairTarget => ActionResult::RequiresRepairTarget,
			ProtoActionResult::NoItemsToDrop => ActionResult::NoItemsToDrop,
			ProtoActionResult::CantHoldAnyMoreItems => ActionResult::CantHoldAnyMoreItems,
			ProtoActionResult::CantHoldThat => ActionResult::CantHoldThat,
			ProtoActionResult::TargetHasNoInventory => ActionResult::TargetHasNoInventory,
			ProtoActionResult::CantDropThisItem => ActionResult::CantDropThisItem,
			ProtoActionResult::CantMoveThisItem => ActionResult::CantMoveThisItem,
			ProtoActionResult::CantPawnThisUnit => ActionResult::CantPawnThisUnit,
			ProtoActionResult::MustTargetCaster => ActionResult::MustTargetCaster,
			ProtoActionResult::CantTargetCaster => ActionResult::CantTargetCaster,
			ProtoActionResult::MustTargetOuter => ActionResult::MustTargetOuter,
			ProtoActionResult::CantTargetOuter => ActionResult::CantTargetOuter,
			ProtoActionResult::MustTargetYourOwnUnits => ActionResult::MustTargetYourOwnUnits,
			ProtoActionResult::CantTargetYourOwnUnits => ActionResult::CantTargetYourOwnUnits,
			ProtoActionResult::MustTargetFriendlyUnits => ActionResult::MustTargetFriendlyUnits,
			ProtoActionResult::CantTargetFriendlyUnits => ActionResult::CantTargetFriendlyUnits,
			ProtoActionResult::MustTargetNeutralUnits => ActionResult::MustTargetNeutralUnits,
			ProtoActionResult::CantTargetNeutralUnits => ActionResult::CantTargetNeutralUnits,
			ProtoActionResult::MustTargetEnemyUnits => ActionResult::MustTargetEnemyUnits,
			ProtoActionResult::CantTargetEnemyUnits => ActionResult::CantTargetEnemyUnits,
			ProtoActionResult::MustTargetAirUnits => ActionResult::MustTargetAirUnits,
			ProtoActionResult::CantTargetAirUnits => ActionResult::CantTargetAirUnits,
			ProtoActionResult::MustTargetGroundUnits => ActionResult::MustTargetGroundUnits,
			ProtoActionResult::CantTargetGroundUnits => ActionResult::CantTargetGroundUnits,
			ProtoActionResult::MustTargetStructures => ActionResult::MustTargetStructures,
			ProtoActionResult::CantTargetStructures => ActionResult::CantTargetStructures,
			ProtoActionResult::MustTargetLightUnits => ActionResult::MustTargetLightUnits,
			ProtoActionResult::CantTargetLightUnits => ActionResult::CantTargetLightUnits,
			ProtoActionResult::MustTargetArmoredUnits => ActionResult::MustTargetArmoredUnits,
			ProtoActionResult::CantTargetArmoredUnits => ActionResult::CantTargetArmoredUnits,
			ProtoActionResult::MustTargetBiologicalUnits => ActionResult::MustTargetBiologicalUnits,
			ProtoActionResult::CantTargetBiologicalUnits => ActionResult::CantTargetBiologicalUnits,
			ProtoActionResult::MustTargetHeroicUnits => ActionResult::MustTargetHeroicUnits,
			ProtoActionResult::CantTargetHeroicUnits => ActionResult::CantTargetHeroicUnits,
			ProtoActionResult::MustTargetRoboticUnits => ActionResult::MustTargetRoboticUnits,
			ProtoActionResult::CantTargetRoboticUnits => ActionResult::CantTargetRoboticUnits,
			ProtoActionResult::MustTargetMechanicalUnits => ActionResult::MustTargetMechanicalUnits,
			ProtoActionResult::CantTargetMechanicalUnits => ActionResult::CantTargetMechanicalUnits,
			ProtoActionResult::MustTargetPsionicUnits => ActionResult::MustTargetPsionicUnits,
			ProtoActionResult::CantTargetPsionicUnits => ActionResult::CantTargetPsionicUnits,
			ProtoActionResult::MustTargetMassiveUnits => ActionResult::MustTargetMassiveUnits,
			ProtoActionResult::CantTargetMassiveUnits => ActionResult::CantTargetMassiveUnits,
			ProtoActionResult::MustTargetMissile => ActionResult::MustTargetMissile,
			ProtoActionResult::CantTargetMissile => ActionResult::CantTargetMissile,
			ProtoActionResult::MustTargetWorkerUnits => ActionResult::MustTargetWorkerUnits,
			ProtoActionResult::CantTargetWorkerUnits => ActionResult::CantTargetWorkerUnits,
			ProtoActionResult::MustTargetEnergyCapableUnits => ActionResult::MustTargetEnergyCapableUnits,
			ProtoActionResult::CantTargetEnergyCapableUnits => ActionResult::CantTargetEnergyCapableUnits,
			ProtoActionResult::MustTargetShieldCapableUnits => ActionResult::MustTargetShieldCapableUnits,
			ProtoActionResult::CantTargetShieldCapableUnits => ActionResult::CantTargetShieldCapableUnits,
			ProtoActionResult::MustTargetFlyers => ActionResult::MustTargetFlyers,
			ProtoActionResult::CantTargetFlyers => ActionResult::CantTargetFlyers,
			ProtoActionResult::MustTargetBuriedUnits => ActionResult::MustTargetBuriedUnits,
			ProtoActionResult::CantTargetBuriedUnits => ActionResult::CantTargetBuriedUnits,
			ProtoActionResult::MustTargetCloakedUnits => ActionResult::MustTargetCloakedUnits,
			ProtoActionResult::CantTargetCloakedUnits => ActionResult::CantTargetCloakedUnits,
			ProtoActionResult::MustTargetUnitsInAStasisField => ActionResult::MustTargetUnitsInAStasisField,
			ProtoActionResult::CantTargetUnitsInAStasisField => ActionResult::CantTargetUnitsInAStasisField,
			ProtoActionResult::MustTargetUnderConstructionUnits => {
				ActionResult::MustTargetUnderConstructionUnits
			}
			ProtoActionResult::CantTargetUnderConstructionUnits => {
				ActionResult::CantTargetUnderConstructionUnits
			}
			ProtoActionResult::MustTargetDeadUnits => ActionResult::MustTargetDeadUnits,
			ProtoActionResult::CantTargetDeadUnits => ActionResult::CantTargetDeadUnits,
			ProtoActionResult::MustTargetRevivableUnits => ActionResult::MustTargetRevivableUnits,
			ProtoActionResult::CantTargetRevivableUnits => ActionResult::CantTargetRevivableUnits,
			ProtoActionResult::MustTargetHiddenUnits => ActionResult::MustTargetHiddenUnits,
			ProtoActionResult::CantTargetHiddenUnits => ActionResult::CantTargetHiddenUnits,
			ProtoActionResult::CantRechargeOtherPlayersUnits => ActionResult::CantRechargeOtherPlayersUnits,
			ProtoActionResult::MustTargetHallucinations => ActionResult::MustTargetHallucinations,
			ProtoActionResult::CantTargetHallucinations => ActionResult::CantTargetHallucinations,
			ProtoActionResult::MustTargetInvulnerableUnits => ActionResult::MustTargetInvulnerableUnits,
			ProtoActionResult::CantTargetInvulnerableUnits => ActionResult::CantTargetInvulnerableUnits,
			ProtoActionResult::MustTargetDetectedUnits => ActionResult::MustTargetDetectedUnits,
			ProtoActionResult::CantTargetDetectedUnits => ActionResult::CantTargetDetectedUnits,
			ProtoActionResult::CantTargetUnitWithEnergy => ActionResult::CantTargetUnitWithEnergy,
			ProtoActionResult::CantTargetUnitWithShields => ActionResult::CantTargetUnitWithShields,
			ProtoActionResult::MustTargetUncommandableUnits => ActionResult::MustTargetUncommandableUnits,
			ProtoActionResult::CantTargetUncommandableUnits => ActionResult::CantTargetUncommandableUnits,
			ProtoActionResult::MustTargetPreventDefeatUnits => ActionResult::MustTargetPreventDefeatUnits,
			ProtoActionResult::CantTargetPreventDefeatUnits => ActionResult::CantTargetPreventDefeatUnits,
			ProtoActionResult::MustTargetPreventRevealUnits => ActionResult::MustTargetPreventRevealUnits,
			ProtoActionResult::CantTargetPreventRevealUnits => ActionResult::CantTargetPreventRevealUnits,
			ProtoActionResult::MustTargetPassiveUnits => ActionResult::MustTargetPassiveUnits,
			ProtoActionResult::CantTargetPassiveUnits => ActionResult::CantTargetPassiveUnits,
			ProtoActionResult::MustTargetStunnedUnits => ActionResult::MustTargetStunnedUnits,
			ProtoActionResult::CantTargetStunnedUnits => ActionResult::CantTargetStunnedUnits,
			ProtoActionResult::MustTargetSummonedUnits => ActionResult::MustTargetSummonedUnits,
			ProtoActionResult::CantTargetSummonedUnits => ActionResult::CantTargetSummonedUnits,
			ProtoActionResult::MustTargetUser1 => ActionResult::MustTargetUser1,
			ProtoActionResult::CantTargetUser1 => ActionResult::CantTargetUser1,
			ProtoActionResult::MustTargetUnstoppableUnits => ActionResult::MustTargetUnstoppableUnits,
			ProtoActionResult::CantTargetUnstoppableUnits => ActionResult::CantTargetUnstoppableUnits,
			ProtoActionResult::MustTargetResistantUnits => ActionResult::MustTargetResistantUnits,
			ProtoActionResult::CantTargetResistantUnits => ActionResult::CantTargetResistantUnits,
			ProtoActionResult::MustTargetDazedUnits => ActionResult::MustTargetDazedUnits,
			ProtoActionResult::CantTargetDazedUnits => ActionResult::CantTargetDazedUnits,
			ProtoActionResult::CantLockdown => ActionResult::CantLockdown,
			ProtoActionResult::CantMindControl => ActionResult::CantMindControl,
			ProtoActionResult::MustTargetDestructibles => ActionResult::MustTargetDestructibles,
			ProtoActionResult::CantTargetDestructibles => ActionResult::CantTargetDestructibles,
			ProtoActionResult::MustTargetItems => ActionResult::MustTargetItems,
			ProtoActionResult::CantTargetItems => ActionResult::CantTargetItems,
			ProtoActionResult::NoCalldownAvailable => ActionResult::NoCalldownAvailable,
			ProtoActionResult::WaypointListFull => ActionResult::WaypointListFull,
			ProtoActionResult::MustTargetRace => ActionResult::MustTargetRace,
			ProtoActionResult::CantTargetRace => ActionResult::CantTargetRace,
			ProtoActionResult::MustTargetSimilarUnits => ActionResult::MustTargetSimilarUnits,
			ProtoActionResult::CantTargetSimilarUnits => ActionResult::CantTargetSimilarUnits,
			ProtoActionResult::CantFindEnoughTargets => ActionResult::CantFindEnoughTargets,
			ProtoActionResult::AlreadySpawningLarva => ActionResult::AlreadySpawningLarva,
			ProtoActionResult::CantTargetExhaustedResources => ActionResult::CantTargetExhaustedResources,
			ProtoActionResult::CantUseMinimap => ActionResult::CantUseMinimap,
			ProtoActionResult::CantUseInfoPanel => ActionResult::CantUseInfoPanel,
			ProtoActionResult::OrderQueueIsFull => ActionResult::OrderQueueIsFull,
			ProtoActionResult::CantHarvestThatResource => ActionResult::CantHarvestThatResource,
			ProtoActionResult::HarvestersNotRequired => ActionResult::HarvestersNotRequired,
			ProtoActionResult::AlreadyTargeted => ActionResult::AlreadyTargeted,
			ProtoActionResult::CantAttackWeaponsDisabled => ActionResult::CantAttackWeaponsDisabled,
			ProtoActionResult::CouldntReachTarget => ActionResult::CouldntReachTarget,
			ProtoActionResult::TargetIsOutOfRange => ActionResult::TargetIsOutOfRange,
			ProtoActionResult::TargetIsTooClose => ActionResult::TargetIsTooClose,
			ProtoActionResult::TargetIsOutOfArc => ActionResult::TargetIsOutOfArc,
			ProtoActionResult::CantFindTeleportLocation => ActionResult::CantFindTeleportLocation,
			ProtoActionResult::InvalidItemClass => ActionResult::InvalidItemClass,
			ProtoActionResult::CantFindCancelOrder => ActionResult::CantFindCancelOrder,
		}
	}
}
