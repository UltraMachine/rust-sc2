use crate::{
	bot::Bot,
	game_data::GameData,
	game_info::GameInfo,
	game_state::GameState,
	ids::AbilityId,
	paths::{get_latest_base_version, get_path_to_sc2, get_version_info},
	player::Computer,
	FromProto, FromProtoData, IntoProto, IntoSC2, Player, PlayerSettings,
};
use num_traits::FromPrimitive;
use protobuf::Message;
use sc2_proto::{
	query::RequestQueryAvailableAbilities,
	sc2api::{PlayerSetup, PlayerType, PortSet, Request, RequestCreateGame, Response, Status},
};
use std::{
	error::Error,
	fmt, fs,
	net::TcpListener,
	ops::{Deref, DerefMut},
	process::{Child, Command},
	rc::Rc,
};
use tungstenite::{client::AutoStream, connect, Message::Binary, WebSocket};
use url::Url;

pub type WS = WebSocket<AutoStream>;
pub type SC2Result<T> = Result<T, Box<dyn Error>>;

const HOST: &str = "127.0.0.1";

#[derive(Debug)]
struct ProtoError(String);
impl ProtoError {
	fn new<E: fmt::Debug>(error: E, details: &str) -> Self {
		Self(format!("{:?}: {}", error, details))
	}
}
impl fmt::Display for ProtoError {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "{}", self.0)
	}
}
impl Error for ProtoError {}

#[derive(Clone)]
struct Ports {
	shared: i32,
	server: [i32; 2],
	client: Vec<[i32; 2]>,
}

// Runners
pub fn run_vs_computer<B>(
	bot: &mut B,
	computer: Computer,
	map_name: &str,
	sc2_version: Option<&str>,
	realtime: bool,
) -> SC2Result<()>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	debug!("Starting game vs computer");

	let sc2_path = get_path_to_sc2();
	let map_path = format!("{}/Maps/{}.SC2Map", sc2_path, map_name);
	// Check if path exists
	fs::metadata(&map_path).unwrap_or_else(|_| panic!("Path doesn't exists: {}", map_path));
	let port = get_unused_port();
	debug!("Launching SC2 process");
	let mut process = launch_client(&sc2_path, port, sc2_version)?;
	debug!("Connecting to websocket");
	let mut ws = connect_to_websocket(HOST, port)?;

	match {
		let settings = bot.get_player_settings();
		debug!("Sending CreateGame request");
		// Create game
		let mut req = Request::new();
		let req_create_game = req.mut_create_game();

		req_create_game.mut_local_map().set_map_path(map_path);
		/*
		Set Map
			req.mut_create_game().mut_local_map().set_map_path("");
		OR
			req.mut_create_game().set_battlenet_map_name("");
		*/
		create_player_setup(&settings, req_create_game);
		create_computer_setup(computer, req_create_game);
		// req_create_game.set_disable_fog(bool); // Cheat
		// req_create_game.set_random_seed(u32);
		req_create_game.set_realtime(realtime);

		let res = send(&mut ws, req)?;
		let res_create_game = res.get_create_game();
		if res_create_game.has_error() {
			terminate(&mut process, &mut ws)?;
			let err = format!(
				"{:?}: {}",
				res_create_game.get_error(),
				res_create_game.get_error_details()
			);
			error!("{}", err);
			panic!(err);
		}

		debug!("Sending JoinGame request");
		bot.player_id = join_game(&settings, &mut ws, None)?;

		set_static_data(bot, &mut ws)?;

		debug!("Entered main loop");
		// Main loop
		play_first_step(&mut ws, bot, realtime)?;
		let mut iteration = 0;
		while play_step(&mut ws, bot, iteration, realtime)? {
			iteration += 1;
		}
		debug!("Game finished");
		Ok(())
	} {
		Ok(()) => terminate(&mut process, &mut ws),
		err => {
			terminate(&mut process, &mut ws)?;
			err
		}
	}
}

pub fn run_ladder_game<B>(
	bot: &mut B,
	host: &str,
	port: &str,
	player_port: i32,
	opponent_id: Option<&str>,
) -> SC2Result<()>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	debug!("Starting ladder game");

	debug!("Connecting to websocket");
	let mut ws = connect_to_websocket(&host, port.parse()?)?;

	debug!("Sending JoinGame request");

	if let Some(id) = opponent_id {
		bot.opponent_id = id.to_string();
	}

	bot.player_id = join_game(
		&bot.get_player_settings(),
		&mut ws,
		Some(Ports {
			shared: player_port + 1,
			server: [player_port + 2, player_port + 3],
			client: vec![[player_port + 4, player_port + 5]],
		}),
	)?;

	set_static_data(bot, &mut ws)?;

	debug!("Entered main loop");
	// Main loop
	let mut iteration = 0;
	play_first_step(&mut ws, bot, false)?;
	while play_step(&mut ws, bot, iteration, false)? {
		iteration += 1;
	}
	debug!("Game finished");

	Ok(())
}

pub fn run_vs_human<B>(
	bot: &mut B,
	human: PlayerSettings,
	map_name: &str,
	sc2_version: Option<&str>,
) -> SC2Result<()>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	debug!("Starting human vs bot");
	let ports = get_unused_ports(9);
	let sc2_path = get_path_to_sc2();
	let (port_host, port_client) = (ports[0], ports[1]);

	debug!("Launching host SC2 process");
	let mut process_host = launch_client(&sc2_path, port_host, sc2_version)?;
	debug!("Launching client SC2 process");
	let mut process_client = launch_client(&sc2_path, port_client, sc2_version)?;
	debug!("Connecting to host websocket");
	let mut ws_host = connect_to_websocket(HOST, port_host)?;
	debug!("Connecting to client websocket");
	let mut ws_client = connect_to_websocket(HOST, port_client)?;

	match {
		debug!("Sending CreateGame request to host process");
		let mut req = Request::new();
		let req_create_game = req.mut_create_game();
		req_create_game
			.mut_local_map()
			.set_map_path(format!("{}/Maps/{}.SC2Map", sc2_path, map_name));
		create_player_setup(&human, req_create_game);
		create_player_setup(&bot.get_player_settings(), req_create_game);
		// req_create_game.set_disable_fog(bool); // Cheat
		// req_create_game.set_random_seed(u32);
		req_create_game.set_realtime(true);

		let res = send(&mut ws_host, req)?;
		let res_create_game = res.get_create_game();
		if res_create_game.has_error() {
			terminate_both(
				&mut process_host,
				&mut ws_host,
				&mut process_client,
				&mut ws_client,
			)?;
			let err = format!(
				"{:?}: {}",
				res_create_game.get_error(),
				res_create_game.get_error_details()
			);
			error!("{}", err);
			panic!(err);
		}

		debug!("Sending JoinGame request to both processes");
		let ports = Ports {
			shared: ports[2],
			server: [ports[3], ports[4]],
			client: vec![[ports[5], ports[6]], [ports[7], ports[8]]],
		};
		join_game(&human, &mut ws_host, Some(ports.clone()))?;
		bot.player_id = join_game(&bot.get_player_settings(), &mut ws_client, Some(ports))?;

		set_static_data(bot, &mut ws_client)?;

		debug!("Entered main loop");
		play_first_step(&mut ws_client, bot, true)?;
		let mut iteration = 0;
		while play_step(&mut ws_client, bot, iteration, true)? {
			iteration += 1;
		}
		debug!("Game finished");
		Ok(())
	} {
		Ok(()) => terminate_both(
			&mut process_host,
			&mut ws_host,
			&mut process_client,
			&mut ws_client,
		),
		err => {
			terminate_both(
				&mut process_host,
				&mut ws_host,
				&mut process_client,
				&mut ws_client,
			)?;
			err
		}
	}
}

// Mini Helpers
fn get_unused_port() -> i32 {
	(1025..65535)
		.find(|port| TcpListener::bind(("127.0.0.1", *port)).is_ok())
		.expect("Can't find available port") as i32
}

fn get_unused_ports(n: usize) -> Vec<i32> {
	let mut ports = Vec::new();
	let mut founded = 0;
	for port in 1025..65535 {
		if TcpListener::bind(("127.0.0.1", port)).is_ok() {
			ports.push(port as i32);
			founded += 1;
			if founded >= n {
				break;
			}
		}
	}
	ports
}

fn send_request(ws: &mut WS, req: Request) -> SC2Result<()> {
	ws.write_message(Binary(req.write_to_bytes()?))?;
	ws.read_message()?;
	Ok(())
}

pub fn send(ws: &mut WS, req: Request) -> SC2Result<Response> {
	ws.write_message(Binary(req.write_to_bytes()?))?;

	let msg = ws.read_message()?;

	let mut res = Response::new();
	res.merge_from_bytes(msg.into_data().as_slice())?;
	Ok(res)
}

// Helpers
fn terminate(process: &mut Child, ws: &mut WS) -> SC2Result<()> {
	debug!("Sending QuitGame request and terminating SC2 process");

	let mut req = Request::new();
	req.mut_quit();
	send_request(ws, req)?;

	process.kill()?;
	Ok(())
}

fn terminate_both(
	process_host: &mut Child,
	ws_host: &mut WS,
	process_client: &mut Child,
	ws_client: &mut WS,
) -> SC2Result<()> {
	debug!("Sending LeaveGame and QuitGame requests and terminating both SC2 processes");

	let mut req = Request::new();
	req.mut_leave_game();
	send_request(ws_host, req.clone())?;
	send_request(ws_client, req)?;

	let mut req = Request::new();
	req.mut_quit();
	send_request(ws_host, req.clone())?;
	send_request(ws_client, req)?;

	process_host.kill()?;
	process_client.kill()?;
	Ok(())
}

fn set_static_data(bot: &mut Bot, ws: &mut WS) -> SC2Result<()> {
	debug!("Requesting GameInfo");
	let mut req = Request::new();
	req.mut_game_info();
	let res = send(ws, req)?;
	bot.game_info = GameInfo::from_proto(res.get_game_info().clone());

	debug!("Requesting GameData");
	let mut req = Request::new();
	let req_game_data = req.mut_data();
	req_game_data.set_ability_id(true);
	req_game_data.set_unit_type_id(true);
	req_game_data.set_upgrade_id(true);
	req_game_data.set_buff_id(true);
	req_game_data.set_effect_id(true);
	let res = send(ws, req)?;
	bot.game_data = Rc::new(GameData::from_proto(res.get_data().clone()));
	Ok(())
}

fn create_player_setup(settings: &PlayerSettings, req_create_game: &mut RequestCreateGame) {
	let mut setup = PlayerSetup::new();

	setup.set_race(settings.race.into_proto());
	setup.set_field_type(PlayerType::Participant);
	if let Some(name) = &settings.name {
		setup.set_player_name(name.clone());
	}
	req_create_game.mut_player_setup().push(setup);
}

fn create_computer_setup(computer: Computer, req_create_game: &mut RequestCreateGame) {
	let mut setup = PlayerSetup::new();

	setup.set_race(computer.race.into_proto());
	setup.set_field_type(PlayerType::Computer);
	setup.set_difficulty(computer.difficulty.into_proto());
	if let Some(ai_build) = computer.ai_build {
		setup.set_ai_build(ai_build.into_proto());
	}
	req_create_game.mut_player_setup().push(setup);
}

fn join_game(settings: &PlayerSettings, ws: &mut WS, ports: Option<Ports>) -> SC2Result<u32> {
	let mut req = Request::new();
	let req_join_game = req.mut_join_game();

	req_join_game.set_race(settings.race.into_proto());

	let options = req_join_game.mut_options();
	options.set_raw(true);
	options.set_score(true);
	// options.mut_feature_layer()
	// options.mut_render();
	options.set_show_cloaked(true);
	options.set_show_burrowed_shadows(true);
	options.set_show_placeholders(true);
	// options.set_raw_affects_selection(bool);
	// options.set_raw_crop_to_playable_area(bool);
	if let Some(name) = &settings.name {
		req_join_game.set_player_name(name.clone());
	}

	if let Some(ports) = ports {
		// Deprecated
		req_join_game.set_shared_port(ports.shared);

		let server_ports = req_join_game.mut_server_ports();
		server_ports.set_game_port(ports.server[0]);
		server_ports.set_base_port(ports.server[1]);

		let client_ports = req_join_game.mut_client_ports();
		ports.client.iter().for_each(|client| {
			let mut port_set = PortSet::new();
			port_set.set_game_port(client[0]);
			port_set.set_base_port(client[1]);
			client_ports.push(port_set);
		});
	}

	let res = send(ws, req)?;
	let res_join_game = res.get_join_game();
	if res_join_game.has_error() {
		let err = ProtoError::new(res_join_game.get_error(), res_join_game.get_error_details());
		error!("{}", err);
		Err(Box::new(err))
	} else {
		Ok(res_join_game.get_player_id())
	}
}

fn play_first_step<B>(ws: &mut WS, bot: &mut B, realtime: bool) -> SC2Result<()>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	let mut req = Request::new();
	req.mut_observation();

	let res = send(ws, req)?;

	bot.init_data_for_unit();
	bot.state = GameState::from_proto_data(bot.get_data_for_unit(), res.get_observation());
	bot.prepare_start(ws);

	bot.on_start(ws)?;

	let bot_actions = bot.get_actions();
	if !bot_actions.is_empty() {
		let mut req = Request::new();
		let actions = req.mut_action().mut_actions();
		bot_actions.into_iter().for_each(|a| actions.push(a.into_proto()));
		bot.clear_actions();
		send_request(ws, req)?;
	}
	if !realtime {
		let mut req = Request::new();
		req.mut_step().set_count(bot.game_step);
		send_request(ws, req)?;
	}
	Ok(())
}

fn play_step<B>(ws: &mut WS, bot: &mut B, iteration: usize, realtime: bool) -> SC2Result<bool>
where
	B: Player + DerefMut<Target = Bot> + Deref<Target = Bot>,
{
	let mut req = Request::new();
	req.mut_observation();

	let res = send(ws, req)?;

	if matches!(res.get_status(), Status::ended) {
		let result = res.get_observation().get_player_result()[bot.player_id as usize]
			.get_result()
			.into_sc2();
		debug!("Result for bot: {:?}", result);
		bot.on_end(result)?;
		return Ok(false);
	}

	let state = GameState::from_proto_data(bot.get_data_for_unit(), res.get_observation());

	let mut req = Request::new();
	let req_query_abilities = req.mut_query().mut_abilities();
	state.observation.raw.units.iter().for_each(|u| {
		if u.is_mine() {
			let mut req_unit = RequestQueryAvailableAbilities::new();
			req_unit.set_unit_tag(u.tag);
			req_query_abilities.push(req_unit);
		}
	});

	let res = send(ws, req)?;
	bot.abilities_units = res
		.get_query()
		.get_abilities()
		.iter()
		.map(|a| {
			(
				a.get_unit_tag(),
				a.get_abilities()
					.iter()
					.filter_map(|ab| AbilityId::from_i32(ab.get_ability_id()))
					.collect(),
			)
		})
		.collect();

	bot.state = state;
	bot.prepare_step();

	bot.on_step(ws, iteration)?;

	let bot_actions = bot.get_actions();
	if !bot_actions.is_empty() {
		// println!("{:?}: {:?}", iteration, bot_actions);
		let mut req = Request::new();
		let actions = req.mut_action().mut_actions();
		bot_actions.into_iter().for_each(|a| actions.push(a.into_proto()));
		bot.clear_actions();
		send_request(ws, req)?;
		/*
		let res = send(ws, req);
		let results = res.get_action().get_result();
		if !results.is_empty() {
			println!("action_results: {:?}", results);
		}
		*/
	}

	let bot_debug_commands = bot.get_debug_commands();
	if !bot_debug_commands.is_empty() {
		let mut req = Request::new();
		let debug_commands = req.mut_debug().mut_debug();
		bot_debug_commands
			.into_iter()
			.for_each(|cmd| debug_commands.push(cmd.into_proto()));
		bot.clear_debug_commands();
		send_request(ws, req)?;
	}
	if !realtime {
		let mut req = Request::new();
		req.mut_step().set_count(bot.game_step);
		send_request(ws, req)?;
	}
	Ok(true)
}

fn launch_client(sc2_path: &str, port: i32, sc2_version: Option<&str>) -> SC2Result<Child> {
	let (base_version, data_hash) = match sc2_version {
		Some(ver) => get_version_info(ver),
		None => (get_latest_base_version(sc2_path), ""),
	};
	let sc2_binary = if cfg!(target_os = "windows") {
		"SC2_x64.exe"
	} else if cfg!(target_os = "linux") {
		"SC2_x64"
	} else {
		panic!("Unsupported OS")
	};

	let mut process = Command::new(format!(
		"{}/Versions/Base{}/{}",
		sc2_path, base_version, sc2_binary
	));
	process
		.current_dir(format!("{}/Support64", sc2_path))
		.arg("-listen")
		.arg(HOST)
		.arg("-port")
		.arg(port.to_string())
		// 0 - windowed, 1 - fullscreen
		.arg("-displayMode")
		.arg("0");
	if !data_hash.is_empty() {
		process.arg("-dataVersion").arg(data_hash);
	}
	let process = process.spawn().expect("Can't launch SC2 process.");
	Ok(process)
}

fn connect_to_websocket(host: &str, port: i32) -> SC2Result<WS> {
	let url = Url::parse(format!("ws://{}:{}/sc2api", host, port).as_str())?;
	let (ws, _rs) = loop {
		if let Ok(result) = connect(url.clone()) {
			break result;
		}
	};
	Ok(ws)
}
