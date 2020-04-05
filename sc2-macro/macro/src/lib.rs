#[macro_use]
extern crate quote;

use proc_macro::TokenStream;
use syn::{
	parse_macro_input, AttributeArgs, Data, DeriveInput, Expr, Fields, ItemFn, ItemImpl, ItemStruct, Meta,
	NestedMeta, Stmt,
};

#[proc_macro_attribute]
pub fn bot(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let attr = parse_macro_input!(attr as AttributeArgs);
	let item = parse_macro_input!(item as ItemStruct);

	let name = item.ident;
	let vis = item.vis;
	let attrs = item.attrs;
	let generics = item.generics;
	let fields = match item.fields {
		Fields::Named(named_fields) => {
			let named = named_fields.named;
			quote! {#named}
		}
		Fields::Unnamed(_) => panic!("#[bot] is not allowed for tuple structs"),
		unit @ Fields::Unit => quote! {#unit},
	};

	TokenStream::from(quote! {
		use rust_sc2::{
			action::{Action, ActionResult, Command, Target},
			constants::{RaceValues, RACE_VALUES},
			debug::{DebugCommand, Debugger},
			game_data::{Cost, GameData},
			game_info::GameInfo,
			game_state::{Alliance, GameState},
			ids::{AbilityId, UnitTypeId, UpgradeId},
			iproduct,
			player::Race,
			query::QueryMaster,
			unit::{DataForUnit, Unit},
			units::{GroupedUnits, Units},
			Itertools, WS,
		};
		use std::{collections::HashMap, rc::Rc};
		#(#attrs)*
		#[derive(Clone)]
		#vis struct #name#generics {
			#fields
			race: Race,
			enemy_race: Race,
			player_id: u32,
			opponent_id: String,
			actions: Vec<Action>,
			commands: HashMap<(AbilityId, Target, bool), Vec<u64>>,
			debug: Debugger,
			query: QueryMaster,
			game_step: u32,
			game_info: GameInfo,
			game_data: Rc<GameData>,
			state: GameState,
			race_values: Rc<RaceValues>,
			data_for_unit: Rc<DataForUnit>,
			grouped_units: GroupedUnits,
			abilities_units: HashMap<u64, Vec<AbilityId>>,
			orders: HashMap<AbilityId, usize>,
			current_units: HashMap<UnitTypeId, usize>,
			time: f32,
			minerals: u32,
			vespene: u32,
			supply_army: u32,
			supply_workers: u32,
			supply_cap: u32,
			supply_used: u32,
			supply_left: u32,
			start_location: Point2,
			enemy_start: Point2,
			techlab_tags: Rc<Vec<u64>>,
			reactor_tags: Rc<Vec<u64>>,
			expansions: Vec<(Point2, Point2)>,
		}
	})
}

#[proc_macro_attribute]
pub fn bot_new(_attr: TokenStream, item: TokenStream) -> TokenStream {
	// let attr = parse_macro_input!(attr as AttributeArgs);
	let item = parse_macro_input!(item as ItemFn);

	let vis = item.vis;
	let signature = item.sig;
	let blocks = item.block.stmts.iter().map(|s| match s {
		Stmt::Expr(expr) => match expr {
			Expr::Struct(struct_expr) => {
				let path = &struct_expr.path;
				let rest = match &struct_expr.rest {
					Some(expr) => quote! {#expr},
					None => quote! {},
				};
				let fields = struct_expr.fields.iter();

				quote! {
					#path {
						#(#fields,)*
						race: Race::Random,
						enemy_race: Race::Random,
						player_id: Default::default(),
						opponent_id: Default::default(),
						actions: Vec::new(),
						commands: HashMap::new(),
						debug: Debugger::new(),
						query: QueryMaster,
						game_info: Default::default(),
						game_data: Default::default(),
						state: Default::default(),
						race_values: Default::default(),
						data_for_unit: Default::default(),
						grouped_units: Default::default(),
						abilities_units: HashMap::new(),
						orders: HashMap::new(),
						current_units: HashMap::new(),
						time: Default::default(),
						minerals: Default::default(),
						vespene: Default::default(),
						supply_army: Default::default(),
						supply_workers: Default::default(),
						supply_cap: Default::default(),
						supply_used: Default::default(),
						supply_left: Default::default(),
						start_location: Default::default(),
						enemy_start: Default::default(),
						techlab_tags: Default::default(),
						reactor_tags: Default::default(),
						expansions: Vec::new(),
						#rest
					}
				}
			}
			_ => panic!("Method `new` must return bot object"),
		},
		n => quote! {#n},
	});

	TokenStream::from(quote! {
		#vis #signature {
			#(#blocks)*
		}
	})
}

#[proc_macro_attribute]
pub fn bot_impl_player(attr: TokenStream, item: TokenStream) -> TokenStream {
	let _attr = parse_macro_input!(attr as AttributeArgs);
	let item = parse_macro_input!(item as ItemImpl);

	let trait_name = match item.trait_ {
		Some((_, trait_name, _)) => trait_name,
		None => unreachable!(),
	};
	let struct_name = item.self_ty;
	let items = item.items;

	TokenStream::from(quote! {
		impl #trait_name for #struct_name {
			#(#items)*
			fn get_step_size(&self) -> u32 {
				self.game_step
			}
			fn set_player_id(&mut self, player_id: u32) {
				self.player_id = player_id;
			}
			fn set_opponent_id(&mut self, opponent_id: String) {
				self.opponent_id = opponent_id;
			}
			fn set_game_info(&mut self, game_info: GameInfo) {
				self.game_info = game_info;
			}
			fn set_game_data(&mut self, game_data: GameData) {
				self.game_data = Rc::new(game_data);
			}
			fn set_state(&mut self, state: GameState) {
				self.state = state;
			}
			fn set_avaliable_abilities(&mut self, abilities_units: HashMap<u64, Vec<AbilityId>>) {
				self.abilities_units = abilities_units;
			}
			fn get_data_for_unit(&self) -> Rc<DataForUnit> {
				Rc::clone(&self.data_for_unit)
			}
			fn get_actions(&self) -> Vec<Action> {
				if !self.commands.is_empty() {
					let mut actions = self.actions.clone();
					self.commands
						.iter()
						.for_each(|((ability, target, queue), units)| {
							actions.push(Action::UnitCommand(*ability, *target, units.clone(), *queue));
						});
					actions
				} else {
					self.actions.clone()
				}
			}
			fn clear_actions(&mut self) {
				self.actions.clear();
				self.commands.clear();
			}
			fn get_debug_commands(&self) -> Vec<DebugCommand> {
				self.debug.get_commands()
			}
			fn clear_debug_commands(&mut self) {
				self.debug.clear_commands();
			}
			fn substract_resources(&mut self, unit: UnitTypeId) {
				let cost = self.game_data.units[&unit].cost();
				self.minerals = self.minerals.saturating_sub(cost.minerals);
				self.vespene = self.vespene.saturating_sub(cost.vespene);
				let supply_cost = cost.supply as u32;
				self.supply_used += supply_cost;
				self.supply_left = self.supply_left.saturating_sub(supply_cost);
			}
			fn substract_upgrade_cost(&mut self, upgrade: UpgradeId) {
				let cost = self.game_data.upgrades[&upgrade].cost();
				self.minerals = self.minerals.saturating_sub(cost.minerals);
				self.vespene = self.vespene.saturating_sub(cost.vespene);
			}
			fn has_upgrade(&self, upgrade: UpgradeId) -> bool {
				self.state.observation.raw.upgrades.contains(&upgrade)
			}
			fn command(&mut self, cmd: Option<Command>) {
				if let Some((tag, order)) = cmd {
					self.commands.entry(order).or_default().push(tag);
				}
			}
			fn chat_send(&mut self, message: String, team_only: bool) {
				self.actions.push(Action::Chat(message, team_only));
			}
			fn init_data_for_unit(&mut self) {
				self.data_for_unit = Rc::new(DataForUnit {
					game_data: Rc::clone(&self.game_data),
					techlab_tags: Rc::clone(&self.techlab_tags),
					reactor_tags: Rc::clone(&self.reactor_tags),
					race_values: Rc::clone(&self.race_values),
				});
			}
			fn prepare_start(&mut self) {
				self.race = self.game_info.players[&self.player_id].race_actual.unwrap();
				if self.game_info.players.len() == 2 {
					self.enemy_race = self.game_info.players[&(3 - self.player_id)].race_requested;
				}
				self.race_values = Rc::new(RACE_VALUES[&self.race].clone());

				self.group_units();

				self.start_location = self.grouped_units.townhalls[0].position;
				self.enemy_start = self.game_info.start_locations[0];

				// Calculating expansion locations
				let geyser_tags = self
					.grouped_units
					.vespene_geysers
					.iter()
					.map(|u| u.tag)
					.collect::<Vec<u64>>();
				let resource_groups = self
					.grouped_units
					.resources
					.iter()
					.filter_map(|u| {
						if u.type_id != UnitTypeId::MineralField450 {
							Some(vec![u])
						} else {
							None
						}
					})
					.combinations(2)
					.filter_map(|res| {
						let res1 = &res[0];
						let res2 = &res[1];
						if iproduct!(res1, res2)
							.any(|(r1, r2)| r1.distance_squared(r2) < 121.0)
						{
							let mut res = res1.clone();
							res.extend(res2);
							Some(res)
						} else {
							None
						}
					})
					.collect::<Vec<Vec<&Unit>>>();

				let offsets = iproduct!(-7..=7, -7..=7)
					.filter(|(x, y)| x * x + y * y <= 64)
					.collect::<Vec<(isize, isize)>>();

				self.expansions = resource_groups
					.iter()
					.map(|res| {
						let res = res.iter().cloned().cloned().collect::<Units>();
						let center = res.center() + 0.5;
						let f = |pos: Point2| res.iter().map(|r| r.distance_pos_squared(pos)).sum::<f32>();
						(
							offsets
								.iter()
								.map(|(x, y)| Point2::new(center.x + (*x as f32), center.y + (*y as f32)))
								.filter(|pos| {
									res.iter().all(|r| {
										r.distance_pos_squared(*pos) > {
											if geyser_tags.contains(&r.tag) {
												49.0
											} else {
												36.0
											}
										}
									})
								})
								.min_by(|pos1, pos2| f(*pos1).partial_cmp(&f(*pos2)).unwrap())
								.unwrap(),
							center,
						)
					})
					.collect::<Vec<(Point2, Point2)>>();
			}
			fn prepare_step(&mut self) {
				self.group_units();
				let observation = &self.state.observation;
				self.time = (observation.game_loop as f32) / 22.4;
				let common = &observation.common;
				self.minerals = common.minerals;
				self.vespene = common.vespene;
				self.supply_army = common.food_army;
				self.supply_workers = common.food_workers;
				self.supply_cap = common.food_cap;
				self.supply_used = common.food_used;
				self.supply_left = self.supply_cap - self.supply_used;

				// Counting units and orders
				self.current_units.clear();
				self.orders.clear();
				self.grouped_units.owned.clone().iter().for_each(|u| {
					u.orders
						.iter()
						.for_each(|order| *self.orders.entry(order.ability).or_default() += 1);
					if !u.is_ready() {
						if u.race() != Race::Terran || !u.is_structure() {
							if let Some(data) = self.game_data.units.get(&u.type_id) {
								if let Some(ability) = data.ability {
									*self.orders.entry(ability).or_default() += 1
								}
							}
						}
					} else {
						*self.current_units.entry(u.type_id).or_default() += 1
					}
				});
			}
			fn group_units(&mut self) {
				let mut owned = Units::new();
				let mut units = Units::new();
				let mut structures = Units::new();
				let mut townhalls = Units::new();
				let mut workers = Units::new();
				let mut enemies = Units::new();
				let mut enemy_units = Units::new();
				let mut enemy_structures = Units::new();
				let mut enemy_townhalls = Units::new();
				let mut enemy_workers = Units::new();
				let mut mineral_fields = Units::new();
				let mut vespene_geysers = Units::new();
				let mut resources = Units::new();
				let mut destructables = Units::new();
				let mut watchtowers = Units::new();
				let mut inhibitor_zones = Units::new();
				let mut gas_buildings = Units::new();
				let mut larva = Units::new();
				let mut techlab_tags = Vec::new();
				let mut reactor_tags = Vec::new();

				self.state.observation.raw.units.iter().cloned().for_each(|u| {
					let u_type = u.type_id;
					match u.alliance {
						Alliance::Neutral => match u_type {
							UnitTypeId::XelNagaTower => {
								watchtowers.push(u);
							}
							UnitTypeId::RichMineralField
							| UnitTypeId::RichMineralField750
							| UnitTypeId::MineralField
							| UnitTypeId::MineralField450
							| UnitTypeId::MineralField750
							| UnitTypeId::LabMineralField
							| UnitTypeId::LabMineralField750
							| UnitTypeId::PurifierRichMineralField
							| UnitTypeId::PurifierRichMineralField750
							| UnitTypeId::PurifierMineralField
							| UnitTypeId::PurifierMineralField750
							| UnitTypeId::BattleStationMineralField
							| UnitTypeId::BattleStationMineralField750
							| UnitTypeId::MineralFieldOpaque
							| UnitTypeId::MineralFieldOpaque900 => {
								resources.push(u.clone());
								mineral_fields.push(u);
							}
							UnitTypeId::VespeneGeyser
							| UnitTypeId::SpacePlatformGeyser
							| UnitTypeId::RichVespeneGeyser
							| UnitTypeId::ProtossVespeneGeyser
							| UnitTypeId::PurifierVespeneGeyser
							| UnitTypeId::ShakurasVespeneGeyser => {
								resources.push(u.clone());
								vespene_geysers.push(u);
							}
							UnitTypeId::InhibitorZoneSmall
							| UnitTypeId::InhibitorZoneMedium
							| UnitTypeId::InhibitorZoneLarge => {
								inhibitor_zones.push(u);
							}
							_ => {
								destructables.push(u);
							}
						},
						Alliance::Own => {
							owned.push(u.clone());
							if u.is_structure() {
								structures.push(u.clone());
								match u_type {
									UnitTypeId::CommandCenter
									| UnitTypeId::OrbitalCommand
									| UnitTypeId::PlanetaryFortress
									| UnitTypeId::CommandCenterFlying
									| UnitTypeId::OrbitalCommandFlying
									| UnitTypeId::Hatchery
									| UnitTypeId::Lair
									| UnitTypeId::Hive
									| UnitTypeId::Nexus => {
										townhalls.push(u);
									}
									UnitTypeId::Refinery
									| UnitTypeId::RefineryRich
									| UnitTypeId::Assimilator
									| UnitTypeId::AssimilatorRich
									| UnitTypeId::Extractor
									| UnitTypeId::ExtractorRich => {
										gas_buildings.push(u);
									}
									UnitTypeId::TechLab
									| UnitTypeId::BarracksTechLab
									| UnitTypeId::FactoryTechLab
									| UnitTypeId::StarportTechLab => techlab_tags.push(u.tag),

									UnitTypeId::Reactor
									| UnitTypeId::BarracksReactor
									| UnitTypeId::FactoryReactor
									| UnitTypeId::StarportReactor => reactor_tags.push(u.tag),
									_ => {}
								}
							} else {
								units.push(u.clone());
								match u_type {
									UnitTypeId::SCV | UnitTypeId::Probe | UnitTypeId::Drone => {
										workers.push(u);
									}
									UnitTypeId::Larva => {
										larva.push(u);
									}
									_ => {}
								}
							}
						}
						Alliance::Enemy => {
							enemies.push(u.clone());
							if u.is_structure() {
								enemy_structures.push(u.clone());
								if [
									UnitTypeId::CommandCenter,
									UnitTypeId::OrbitalCommand,
									UnitTypeId::PlanetaryFortress,
									UnitTypeId::CommandCenterFlying,
									UnitTypeId::OrbitalCommandFlying,
									UnitTypeId::Hatchery,
									UnitTypeId::Lair,
									UnitTypeId::Hive,
									UnitTypeId::Nexus,
								]
								.contains(&u_type)
								{
									enemy_townhalls.push(u);
								}
							} else {
								enemy_units.push(u.clone());
								if [UnitTypeId::SCV, UnitTypeId::Probe, UnitTypeId::Drone].contains(&u_type) {
									enemy_workers.push(u);
								}
							}
						}
						_ => {}
					}
				});

				self.grouped_units = GroupedUnits {
					owned,
					units,
					structures,
					townhalls,
					workers,
					enemies,
					enemy_units,
					enemy_structures,
					enemy_townhalls,
					enemy_workers,
					mineral_fields,
					vespene_geysers,
					resources,
					destructables,
					watchtowers,
					inhibitor_zones,
					gas_buildings,
					larva,
				};
				self.techlab_tags = Rc::new(techlab_tags);
				self.reactor_tags = Rc::new(reactor_tags);

				self.data_for_unit = Rc::new(DataForUnit {
					game_data: Rc::clone(&self.game_data),
					techlab_tags: Rc::clone(&self.techlab_tags),
					reactor_tags: Rc::clone(&self.reactor_tags),
					race_values: Rc::clone(&self.race_values),
				});
			}
			fn get_unit_cost(&self, unit: UnitTypeId) -> Cost {
				match self.game_data.units.get(&unit) {
					Some(data) => data.cost(),
					None => Default::default(),
				}
			}
			fn can_afford(&self, unit: UnitTypeId, check_supply: bool) -> bool {
				let cost = self.get_unit_cost(unit);
				if self.minerals < cost.minerals || self.vespene < cost.vespene {
					return false;
				}
				if check_supply && (self.supply_left as f32) < cost.supply {
					return false;
				}
				true
			}
			fn get_upgrade_cost(&self, upgrade: UpgradeId) -> Cost {
				match self.game_data.upgrades.get(&upgrade) {
					Some(data) => data.cost(),
					None => Default::default(),
				}
			}
			fn can_afford_upgrade(&self, upgrade: UpgradeId) -> bool {
				let cost = self.get_upgrade_cost(upgrade);
				self.minerals >= cost.minerals && self.vespene >= cost.vespene
			}
			/*
			fn can_afford_ability(&self, ability: AbilityId) -> bool {
				unimplemented!()
			}
			*/
			#[allow(clippy::too_many_arguments)]
			fn find_placement(
				&self,
				ws: &mut WS,
				building: UnitTypeId,
				near: Point2,
				max_distance: isize,
				placement_step: isize,
				random: bool,
				addon: bool,
			) -> Option<Point2> {
				if let Some(data) = self.game_data.units.get(&building) {
					if let Some(ability) = data.ability {
						if self
							.query
							.placement(
								ws,
								if addon {
									vec![
										(ability, near, None),
										(AbilityId::TerranBuildSupplyDepot, near.offset(2.5, -0.5), None),
									]
								} else {
									vec![(ability, near, None)]
								},
								false,
							)
							.unwrap()[0] == ActionResult::Success
						{
							return Some(near);
						}

						for distance in (placement_step..max_distance).step_by(placement_step as usize) {
							let positions = (-distance..=distance)
								.step_by(placement_step as usize)
								.flat_map(|offset| {
									vec![
										near.offset(offset as f32, (-distance) as f32),
										near.offset(offset as f32, distance as f32),
										near.offset((-distance) as f32, offset as f32),
										near.offset(distance as f32, offset as f32),
									]
								})
								.collect::<Vec<Point2>>();
							let results = self
								.query
								.placement(
									ws,
									positions.iter().map(|pos| (ability, *pos, None)).collect(),
									false,
								)
								.unwrap();

							let mut valid_positions = positions
								.iter()
								.zip(results.iter())
								.filter_map(|(pos, res)| {
									if *res == ActionResult::Success {
										Some(*pos)
									} else {
										None
									}
								})
								.collect::<Vec<Point2>>();

							if addon {
								let results = self
									.query
									.placement(
										ws,
										valid_positions
											.iter()
											.map(|pos| {
												(AbilityId::TerranBuildSupplyDepot, pos.offset(2.5, -0.5), None)
											})
											.collect(),
										false,
									)
									.unwrap();
								valid_positions = valid_positions
									.iter()
									.zip(results.iter())
									.filter_map(|(pos, res)| {
										if *res == ActionResult::Success {
											Some(*pos)
										} else {
											None
										}
									})
									.collect::<Vec<Point2>>();
							}

							if !valid_positions.is_empty() {
								return if random {
									let mut rng = thread_rng();
									valid_positions.choose(&mut rng).copied()
								} else {
									let f = |pos: Point2| near.distance_squared(pos);
									valid_positions
										.iter()
										.min_by(|pos1, pos2| f(**pos1).partial_cmp(&f(**pos2)).unwrap())
										.copied()
								};
							}
						}
					}
				}
				None
			}
			fn find_gas_placement(&self, ws: &mut WS, base: Point2) -> Option<Unit> {
				let ability = self.game_data.units[&self.race_values.gas_building]
					.ability
					.unwrap();

				let geysers = self.grouped_units.vespene_geysers.closer_pos(11.0, base);
				let results = self
					.query
					.placement(
						ws,
						geysers.iter().map(|u| (ability, u.position, None)).collect(),
						false,
					)
					.unwrap();

				let valid_geysers = geysers
					.iter()
					.zip(results.iter())
					.filter_map(|(geyser, res)| {
						if *res == ActionResult::Success {
							Some(geyser)
						} else {
							None
						}
					})
					.collect::<Vec<&Unit>>();

				if valid_geysers.is_empty() {
					None
				} else {
					Some(valid_geysers[0].clone())
				}
			}
		}
	})
}

#[proc_macro_derive(FromStr, attributes(enum_from_str))]
pub fn enum_from_str_derive(input: TokenStream) -> TokenStream {
	let item = parse_macro_input!(input as DeriveInput);
	if let Data::Enum(data) = item.data {
		let name = item.ident;
		let variants = data.variants.iter().map(|v| &v.ident);
		let name_iter = vec![name.clone(); variants.len()];

		#[allow(clippy::block_in_if_condition_stmt)]
		let other_cases = if item.attrs.iter().any(|a| {
			if a.path.is_ident("enum_from_str") {
				if let Meta::List(list) = a.parse_meta().unwrap() {
					return list.nested.iter().any(|n| {
						if let NestedMeta::Meta(Meta::Path(path)) = n {
							path.is_ident("use_primitives")
						} else {
							false
						}
					});
				} else {
					unreachable!("No options found in attribute `enum_from_str`")
				}
			}
			false
		}) {
			quote! {
				n => {
					if let Ok(num) = n.parse() {
						if let Some(result) = Self::from_i64(num) {
							return Ok(result);
						}
					}
					return Err(sc2_macro::ParseEnumError);
				}
			}
		} else {
			quote! {_ => return Err(sc2_macro::ParseEnumError)}
		};
		TokenStream::from(quote! {
			impl std::str::FromStr for #name {
				type Err = sc2_macro::ParseEnumError;

				fn from_str(s: &str) -> Result<Self, Self::Err> {
					Ok(match s {
						#(
							stringify!(#variants) => Self::#variants,
							concat!(stringify!(#name_iter), "::", stringify!(#variants)) => Self::#variants,

						)*
						#other_cases,
					})
				}
			}
		})
	} else {
		panic!("Can only derive FromStr for enums")
	}
}
