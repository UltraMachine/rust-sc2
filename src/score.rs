use crate::{FromProto, IntoSC2};
use sc2_proto::score::{CategoryScoreDetails, Score as ProtoScore, Score_ScoreType, VitalScoreDetails};

#[variant_checkers]
#[derive(Clone)]
pub enum ScoreType {
	Curriculum,
	Melee,
}
impl FromProto<Score_ScoreType> for ScoreType {
	fn from_proto(score_type: Score_ScoreType) -> Self {
		match score_type {
			Score_ScoreType::Curriculum => ScoreType::Curriculum,
			Score_ScoreType::Melee => ScoreType::Melee,
		}
	}
}
impl Default for ScoreType {
	fn default() -> Self {
		ScoreType::Curriculum
	}
}

#[derive(Default, Clone)]
pub struct Category {
	pub none: f32,
	pub army: f32,
	pub economy: f32,
	pub technology: f32,
	pub upgrade: f32,
}
impl FromProto<&CategoryScoreDetails> for Category {
	fn from_proto(category: &CategoryScoreDetails) -> Self {
		Self {
			none: category.get_none(),
			army: category.get_army(),
			economy: category.get_economy(),
			technology: category.get_technology(),
			upgrade: category.get_upgrade(),
		}
	}
}

#[derive(Default, Clone)]
pub struct Vital {
	pub life: f32,
	pub shields: f32,
	pub energy: f32,
}
impl FromProto<&VitalScoreDetails> for Vital {
	fn from_proto(vital: &VitalScoreDetails) -> Self {
		Self {
			life: vital.get_life(),
			shields: vital.get_shields(),
			energy: vital.get_energy(),
		}
	}
}

#[derive(Default, Clone)]
pub struct Score {
	pub score_type: ScoreType,
	pub total_score: i32,
	// score details
	pub idle_production_time: f32,
	pub idle_worker_time: f32,
	pub total_value_units: f32,
	pub total_value_structures: f32,
	pub killed_value_units: f32,
	pub killed_value_structures: f32,
	pub collected_minerals: f32,
	pub collected_vespene: f32,
	pub collection_rate_minerals: f32,
	pub collection_rate_vespene: f32,
	pub spent_minerals: f32,
	pub spent_vespene: f32,
	pub food_used: Category,
	pub killed_minerals: Category,
	pub killed_vespene: Category,
	pub lost_minerals: Category,
	pub lost_vespene: Category,
	pub friendly_fire_minerals: Category,
	pub friendly_fire_vespene: Category,
	pub used_minerals: Category,
	pub used_vespene: Category,
	pub total_used_minerals: Category,
	pub total_used_vespene: Category,
	pub total_damage_dealt: Vital,
	pub total_damage_taken: Vital,
	pub total_healed: Vital,
	pub current_apm: f32,
	pub current_effective_apm: f32,
}
impl FromProto<&ProtoScore> for Score {
	fn from_proto(score: &ProtoScore) -> Self {
		let details = score.get_score_details();
		Self {
			score_type: score.get_score_type().into_sc2(),
			total_score: score.get_score(),
			idle_production_time: details.get_idle_production_time(),
			idle_worker_time: details.get_idle_worker_time(),
			total_value_units: details.get_total_value_units(),
			total_value_structures: details.get_total_value_structures(),
			killed_value_units: details.get_killed_value_units(),
			killed_value_structures: details.get_killed_value_structures(),
			collected_minerals: details.get_collected_minerals(),
			collected_vespene: details.get_collected_vespene(),
			collection_rate_minerals: details.get_collection_rate_minerals(),
			collection_rate_vespene: details.get_collection_rate_vespene(),
			spent_minerals: details.get_spent_minerals(),
			spent_vespene: details.get_spent_vespene(),
			food_used: details.get_food_used().into_sc2(),
			killed_minerals: details.get_killed_minerals().into_sc2(),
			killed_vespene: details.get_killed_vespene().into_sc2(),
			lost_minerals: details.get_lost_minerals().into_sc2(),
			lost_vespene: details.get_lost_vespene().into_sc2(),
			friendly_fire_minerals: details.get_friendly_fire_minerals().into_sc2(),
			friendly_fire_vespene: details.get_friendly_fire_vespene().into_sc2(),
			used_minerals: details.get_used_minerals().into_sc2(),
			used_vespene: details.get_used_vespene().into_sc2(),
			total_used_minerals: details.get_total_used_minerals().into_sc2(),
			total_used_vespene: details.get_total_used_vespene().into_sc2(),
			total_damage_dealt: details.get_total_damage_dealt().into_sc2(),
			total_damage_taken: details.get_total_damage_taken().into_sc2(),
			total_healed: details.get_total_healed().into_sc2(),
			current_apm: details.get_current_apm(),
			current_effective_apm: details.get_current_effective_apm(),
		}
	}
}
