use crate::{FromProto, IntoProto};
use num_traits::FromPrimitive;
use sc2_proto::{
	common::Race as ProtoRace,
	sc2api::{AIBuild as ProtoAIBuild, Difficulty as ProtoDifficulty, PlayerType as ProtoPlayerType},
};

#[derive(Copy, Clone, PartialEq, Eq, Hash, FromStr)]
pub enum Race {
	Terran,
	Zerg,
	Protoss,
	Random,
}
impl FromProto<ProtoRace> for Race {
	fn from_proto(race: ProtoRace) -> Self {
		match race {
			ProtoRace::Terran => Race::Terran,
			ProtoRace::Zerg => Race::Zerg,
			ProtoRace::Protoss => Race::Protoss,
			ProtoRace::Random => Race::Random,
			ProtoRace::NoRace => Race::Random,
		}
	}
}
impl IntoProto<ProtoRace> for Race {
	fn into_proto(self) -> ProtoRace {
		match self {
			Race::Terran => ProtoRace::Terran,
			Race::Zerg => ProtoRace::Zerg,
			Race::Protoss => ProtoRace::Protoss,
			Race::Random => ProtoRace::Random,
		}
	}
}
impl Default for Race {
	fn default() -> Self {
		Race::Random
	}
}

#[derive(Copy, Clone, FromPrimitive, FromStr)]
#[enum_from_str(use_primitives)]
pub enum Difficulty {
	VeryEasy,
	Easy,
	Medium,
	MediumHard,
	Hard,
	Harder,
	VeryHard,
	CheatVision,
	CheatMoney,
	CheatInsane,
}
impl FromProto<ProtoDifficulty> for Difficulty {
	fn from_proto(difficulty: ProtoDifficulty) -> Self {
		match difficulty {
			ProtoDifficulty::VeryEasy => Difficulty::VeryEasy,
			ProtoDifficulty::Easy => Difficulty::Easy,
			ProtoDifficulty::Medium => Difficulty::Medium,
			ProtoDifficulty::MediumHard => Difficulty::MediumHard,
			ProtoDifficulty::Hard => Difficulty::Hard,
			ProtoDifficulty::Harder => Difficulty::Harder,
			ProtoDifficulty::VeryHard => Difficulty::VeryHard,
			ProtoDifficulty::CheatVision => Difficulty::CheatVision,
			ProtoDifficulty::CheatMoney => Difficulty::CheatMoney,
			ProtoDifficulty::CheatInsane => Difficulty::CheatInsane,
		}
	}
}
impl IntoProto<ProtoDifficulty> for Difficulty {
	fn into_proto(self) -> ProtoDifficulty {
		match self {
			Difficulty::VeryEasy => ProtoDifficulty::VeryEasy,
			Difficulty::Easy => ProtoDifficulty::Easy,
			Difficulty::Medium => ProtoDifficulty::Medium,
			Difficulty::MediumHard => ProtoDifficulty::MediumHard,
			Difficulty::Hard => ProtoDifficulty::Hard,
			Difficulty::Harder => ProtoDifficulty::Harder,
			Difficulty::VeryHard => ProtoDifficulty::VeryHard,
			Difficulty::CheatVision => ProtoDifficulty::CheatVision,
			Difficulty::CheatMoney => ProtoDifficulty::CheatMoney,
			Difficulty::CheatInsane => ProtoDifficulty::CheatInsane,
		}
	}
}

#[derive(Copy, Clone, FromStr)]
pub enum AIBuild {
	RandomBuild,
	Rush,
	Timing,
	Power,
	Macro,
	Air,
}
impl FromProto<ProtoAIBuild> for AIBuild {
	fn from_proto(ai_build: ProtoAIBuild) -> Self {
		match ai_build {
			ProtoAIBuild::RandomBuild => AIBuild::RandomBuild,
			ProtoAIBuild::Rush => AIBuild::Rush,
			ProtoAIBuild::Timing => AIBuild::Timing,
			ProtoAIBuild::Power => AIBuild::Power,
			ProtoAIBuild::Macro => AIBuild::Macro,
			ProtoAIBuild::Air => AIBuild::Air,
		}
	}
}
impl IntoProto<ProtoAIBuild> for AIBuild {
	fn into_proto(self) -> ProtoAIBuild {
		match self {
			AIBuild::RandomBuild => ProtoAIBuild::RandomBuild,
			AIBuild::Rush => ProtoAIBuild::Rush,
			AIBuild::Timing => ProtoAIBuild::Timing,
			AIBuild::Power => ProtoAIBuild::Power,
			AIBuild::Macro => ProtoAIBuild::Macro,
			AIBuild::Air => ProtoAIBuild::Air,
		}
	}
}
impl Default for AIBuild {
	fn default() -> Self {
		AIBuild::RandomBuild
	}
}

#[derive(Copy, Clone, PartialEq, Eq)]
pub enum PlayerType {
	Participant,
	Computer,
	Observer,
}
impl FromProto<ProtoPlayerType> for PlayerType {
	fn from_proto(player_type: ProtoPlayerType) -> Self {
		match player_type {
			ProtoPlayerType::Participant => PlayerType::Participant,
			ProtoPlayerType::Computer => PlayerType::Computer,
			ProtoPlayerType::Observer => PlayerType::Observer,
		}
	}
}
impl IntoProto<ProtoPlayerType> for PlayerType {
	fn into_proto(self) -> ProtoPlayerType {
		match self {
			PlayerType::Participant => ProtoPlayerType::Participant,
			PlayerType::Computer => ProtoPlayerType::Computer,
			PlayerType::Observer => ProtoPlayerType::Observer,
		}
	}
}

pub struct Computer {
	pub race: Race,
	pub difficulty: Difficulty,
	pub ai_build: Option<AIBuild>,
}
impl Computer {
	pub fn new(race: Race, difficulty: Difficulty, ai_build: Option<AIBuild>) -> Self {
		Self {
			race,
			difficulty,
			ai_build,
		}
	}
}
