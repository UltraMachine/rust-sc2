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
			action::{Action, Command, Target},
			debug::{Debugger, DebugCommand},
			game_data::{GameData, Cost},
			game_info::GameInfo,
			game_state::{Alliance, GameState},
			ids::{AbilityId, UnitTypeId /*, UpgradeId*/},
			units::{GroupedUnits, Units},
		};
		use std::{collections::HashMap, rc::Rc};
		#(#attrs)*
		#[derive(Clone)]
		#vis struct #name#generics {
			#fields
			player_id: u32,
			opponent_id: String,
			actions: Vec<Action>,
			commands: HashMap<(AbilityId, Target, bool), Vec<u64>>,
			debug: Debugger,
			game_step: u32,
			game_info: GameInfo,
			game_data: Rc<GameData>,
			state: GameState,
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
						player_id: Default::default(),
						opponent_id: Default::default(),
						actions: Vec::new(),
						commands: HashMap::new(),
						debug: Debugger::new(),
						game_info: Default::default(),
						game_data: Default::default(),
						state: Default::default(),
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
			fn get_game_data(&self) -> Rc<GameData> {
				Rc::clone(&self.game_data)
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
			fn command(&mut self, cmd: Option<Command>) {
				if let Some((tag, order)) = cmd {
					self.commands.entry(order).or_default().push(tag);
				}
			}
			fn chat_send(&mut self, message: String, team_only: bool) {
				self.actions.push(Action::Chat(message, team_only));
			}
			fn prepare_first_step(&mut self) {
				self.group_units();
				self.start_location = self.grouped_units.townhalls[0].position;
				self.enemy_start = self.game_info.start_locations[0];
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
				let mut mineral_field = Units::new();
				let mut vespene_geyser = Units::new();
				let mut resources = Units::new();
				let mut destructables = Units::new();
				let mut watchtowers = Units::new();
				let mut inhibitor_zones = Units::new();
				let mut gas_buildings = Units::new();
				let mut larva = Units::new();
				// let mut techlab_tags = Vec::new();
				// let mut reactor_tags = Vec::new();

				self.state.observation.raw.units.iter().cloned().for_each(|u| {
					let u_type = u.type_id;
					match u.alliance {
						Alliance::Neutral => match u_type {
							UnitTypeId::XelNagaTower => watchtowers.push(u),

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
								mineral_field.push(u);
							}
							UnitTypeId::VespeneGeyser
							| UnitTypeId::SpacePlatformGeyser
							| UnitTypeId::RichVespeneGeyser
							| UnitTypeId::ProtossVespeneGeyser
							| UnitTypeId::PurifierVespeneGeyser
							| UnitTypeId::ShakurasVespeneGeyser => {
								resources.push(u.clone());
								vespene_geyser.push(u);
							}
							UnitTypeId::InhibitorZoneSmall
							| UnitTypeId::InhibitorZoneMedium
							| UnitTypeId::InhibitorZoneLarge => inhibitor_zones.push(u),

							_ => destructables.push(u),
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
									| UnitTypeId::Nexus => townhalls.push(u),

									UnitTypeId::Refinery
									| UnitTypeId::RefineryRich
									| UnitTypeId::Assimilator
									| UnitTypeId::AssimilatorRich
									| UnitTypeId::Extractor
									| UnitTypeId::ExtractorRich => gas_buildings.push(u),
									_ => {}
								}
							} else {
								units.push(u.clone());
								match u_type {
									UnitTypeId::SCV | UnitTypeId::Probe | UnitTypeId::Drone => workers.push(u),
									UnitTypeId::Larva => larva.push(u),
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
					mineral_field,
					vespene_geyser,
					resources,
					destructables,
					watchtowers,
					inhibitor_zones,
					gas_buildings,
					larva,
				}
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
			/*
			fn can_afford_upgrade(&self, upgrade: UpgradeId) -> bool {
				unimplemented!()
			}
			fn can_afford_ability(&self, ability: AbilityId) -> bool {
				unimplemented!()
			}
			*/
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
