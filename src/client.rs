use crate::{
	game_data::GameData,
	game_info::GameInfo,
	game_state::GameState,
	ids::AbilityId,
	paths::{get_base_version, get_latest_base_version, get_path_to_sc2},
	player::PlayerType,
	FromProto, FromProtoPlayer, IntoProto, PlayerBox,
};
use num_traits::FromPrimitive;
use protobuf::Message;
use sc2_proto::{
	query::RequestQueryAvailableAbilities,
	sc2api::{PlayerSetup, PortSet, Request, RequestCreateGame, Response, Status},
};
use std::{
	io::Write,
	net::TcpListener,
	process::{Child, Command},
	rc::Rc,
};
use tungstenite::{client::AutoStream, connect, Message::Binary, Result as TResult, WebSocket};
use url::Url;

pub type WS = WebSocket<AutoStream>;

#[derive(Clone)]
struct Ports {
	shared: i32,
	server: [i32; 2],
	client: Vec<[i32; 2]>,
}

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

fn send_request(ws: &mut WS, req: Request) -> TResult<()> {
	ws.write_message(Binary(req.write_to_bytes().unwrap()))?;
	ws.read_message()?;
	Ok(())
}

pub fn send(ws: &mut WS, req: Request) -> TResult<Response> {
	ws.write_message(Binary(req.write_to_bytes().unwrap()))?;

	let msg = ws.read_message()?;

	let mut res = Response::new();
	res.merge_from_bytes(msg.into_data().as_slice()).unwrap();
	Ok(res)
}

fn flush() {
	std::io::stdout().flush().unwrap();
}

fn terminate(process: &mut Child, ws: &mut WS) -> TResult<()> {
	print!("Sending QuitGame request and terminating SC2 process... ");
	flush();

	let mut req = Request::new();
	req.mut_quit();
	send_request(ws, req)?;

	process.kill()?;
	println!("Done");
	Ok(())
}

fn terminate_both(
	process_host: &mut Child,
	ws_host: &mut WS,
	process_client: &mut Child,
	ws_client: &mut WS,
) -> TResult<()> {
	print!("Sending LeaveGame and QuitGame requests and terminating SC2 both processes... ");
	flush();

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
	println!("Done");
	Ok(())
}

pub fn run_game(
	map_name: String,
	players: Vec<PlayerBox>,
	realtime: bool,
	sc2_version: Option<String>,
) -> TResult<()> {
	let mut player1 = players[0].clone();
	let mut player2 = players[1].clone();
	match player1.get_player_settings().player_type {
		PlayerType::Participant | PlayerType::Human => match player2.get_player_settings().player_type {
			PlayerType::Participant | PlayerType::Human => {
				launch_pvp(&mut player1, &mut player2, map_name, realtime, sc2_version)?
			}
			PlayerType::Computer => {
				launch_vs_computer(&mut player1, player2, map_name, realtime, sc2_version)?
			}
			PlayerType::Observer => unimplemented!("Observer is not supported"),
		},
		PlayerType::Computer => match player2.get_player_settings().player_type {
			PlayerType::Participant | PlayerType::Human => {
				launch_vs_computer(&mut player2, player1, map_name, realtime, sc2_version)?
			}
			PlayerType::Computer => panic!("Match between two Computers is not allowed"),
			PlayerType::Observer => unimplemented!("Observer is not supported"),
		},
		PlayerType::Observer => unimplemented!("Observer is not supported"),
	};
	Ok(())
}

pub fn run_ladder_game(
	mut player: PlayerBox,
	host: String,
	port: String,
	player_port: i32,
	opponent_id: Option<&str>,
) -> TResult<()> {
	println!("Starting ladder game.");

	print!("Connecting to websocket... ");
	flush();
	let url = Url::parse(format!("ws://{}:{}/sc2api", host, port).as_str()).unwrap();
	let (mut ws, _rs) = loop {
		if let Ok(result) = connect(url.clone()) {
			break result;
		}
	};
	println!("Done");

	print!("Sending JoinGame request... ");
	flush();

	if let Some(id) = opponent_id {
		player.set_opponent_id(id.to_string());
	}

	join_game(
		&mut player,
		&mut ws,
		Some(Ports {
			shared: player_port + 1,
			server: [player_port + 2, player_port + 3],
			client: vec![[player_port + 4, player_port + 5]],
		}),
	)?;
	println!("Done");

	print!("Requesting GameInfo... ");
	flush();
	let mut req = Request::new();
	req.mut_game_info();
	let res = send(&mut ws, req)?;
	player.set_game_info(GameInfo::from_proto(res.get_game_info().clone()));
	println!("Done");

	print!("Requesting GameData... ");
	flush();
	let mut req = Request::new();
	let req_game_data = req.mut_data();
	req_game_data.set_ability_id(true);
	req_game_data.set_unit_type_id(true);
	req_game_data.set_upgrade_id(true);
	req_game_data.set_buff_id(true);
	req_game_data.set_effect_id(true);

	let res = send(&mut ws, req)?;
	player.set_game_data(GameData::from_proto(res.get_data().clone()));
	println!("Done");

	print!("Entered main loop... ");
	flush();
	// Main loop
	let mut iteration = 0;
	play_first_step(&mut ws, &mut player, false, false)?;
	loop {
		if !play_step(&mut ws, &mut player, iteration, false, false)? {
			break;
		}
		iteration += 1;
	}
	println!("Finished");

	Ok(())
}

fn create_player_setup(p: &PlayerBox, req_create_game: &mut RequestCreateGame) {
	let mut setup = PlayerSetup::new();
	let settings = p.get_player_settings();

	setup.set_race(settings.race.into_proto());
	setup.set_field_type(settings.player_type.into_proto());
	if let Some(name) = settings.name {
		setup.set_player_name(name);
	}
	req_create_game.mut_player_setup().push(setup);
}

fn create_computer_setup(p: &PlayerBox, req_create_game: &mut RequestCreateGame) {
	let mut setup = PlayerSetup::new();
	let settings = p.get_player_settings();

	setup.set_race(settings.race.into_proto());
	setup.set_field_type(settings.player_type.into_proto());
	setup.set_difficulty(settings.difficulty.unwrap().into_proto());
	if let Some(ai_build) = settings.ai_build {
		setup.set_ai_build(ai_build.into_proto());
	}
	req_create_game.mut_player_setup().push(setup);
}

fn join_game(p: &mut PlayerBox, ws: &mut WS, ports: Option<Ports>) -> TResult<()> {
	let mut req = Request::new();
	let req_join_game = req.mut_join_game();

	let player_settings = p.get_player_settings();

	req_join_game.set_race(player_settings.race.into_proto());

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
	if let Some(name) = player_settings.name {
		req_join_game.set_player_name(name);
	}

	if let Some(ports) = ports {
		// Deprecated
		req_join_game.set_shared_port(ports.shared);

		let server_ports = req_join_game.mut_server_ports();
		server_ports.set_game_port(ports.server[0]);
		server_ports.set_base_port(ports.server[1]);

		let client_ports = req_join_game.mut_client_ports();
		for client in ports.client {
			let mut port_set = PortSet::new();
			port_set.set_game_port(client[0]);
			port_set.set_base_port(client[1]);
			client_ports.push(port_set);
		}
	}

	let res = send(ws, req)?;
	let res_join_game = res.get_join_game();
	p.set_player_id(res_join_game.get_player_id());
	if res_join_game.has_error() {
		panic!(
			"{:?}: {}",
			res_join_game.get_error(),
			res_join_game.get_error_details()
		);
	}
	Ok(())
}

fn play_first_step(ws: &mut WS, player: &mut PlayerBox, realtime: bool, human: bool) -> TResult<()> {
	if !human {
		let mut req = Request::new();
		req.mut_observation();

		let res = send(ws, req)?;

		player.init_data_for_unit();
		player.set_state(GameState::from_proto_player(Rc::new(player.clone()), res.get_observation()));
		player.prepare_start(ws);

		player.on_start(ws);

		let player_actions = player.get_actions();
		if !player_actions.is_empty() {
			let mut req = Request::new();
			let actions = req.mut_action().mut_actions();
			player_actions
				.into_iter()
				.for_each(|a| actions.push(a.into_proto()));
			player.clear_actions();
			send_request(ws, req)?;
		}
	}
	if !realtime {
		let mut req = Request::new();
		req.mut_step().set_count(player.get_step_size());
		send_request(ws, req)?;
	}
	Ok(())
}

fn play_step(
	ws: &mut WS,
	player: &mut PlayerBox,
	iteration: usize,
	realtime: bool,
	human: bool,
) -> TResult<bool> {
	if !human {
		let mut req = Request::new();
		req.mut_observation();

		let res = send(ws, req)?;

		if res.get_status() == Status::ended {
			return Ok(false);
		}

		let state = GameState::from_proto_player(Rc::new(player.clone()), res.get_observation());

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
		player.set_avaliable_abilities(
			res.get_query()
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
				.collect(),
		);

		player.set_state(state);
		player.prepare_step();

		player.on_step(ws, iteration);

		let player_actions = player.get_actions();
		if !player_actions.is_empty() {
			// println!("{:?}: {:?}", iteration, player_actions);
			let mut req = Request::new();
			let actions = req.mut_action().mut_actions();
			player_actions
				.into_iter()
				.for_each(|a| actions.push(a.into_proto()));
			player.clear_actions();
			send_request(ws, req)?;
			/*
			let res = send(ws, req);
			let results = res.get_action().get_result();
			if !results.is_empty() {
				println!("action_results: {:?}", results);
			}
			*/
		}

		let player_debug_commands = player.get_debug_commands();
		if !player_debug_commands.is_empty() {
			let mut req = Request::new();
			let debug_commands = req.mut_debug().mut_debug();
			player_debug_commands
				.into_iter()
				.for_each(|cmd| debug_commands.push(cmd.into_proto()));
			player.clear_debug_commands();
			send_request(ws, req)?;
		}
	}
	if !realtime {
		let mut req = Request::new();
		req.mut_step().set_count(player.get_step_size());
		send_request(ws, req)?;
	}
	Ok(true)
}

fn launch_vs_computer(
	player: &mut PlayerBox,
	computer: PlayerBox,
	map_name: String,
	realtime: bool,
	sc2_version: Option<String>,
) -> TResult<()> {
	println!("Starting game vs Computer.");
	// config
	let (host, port): (&str, i32) = ("127.0.0.1", get_unused_port());
	let sc2_path = get_path_to_sc2();
	let base_version = match sc2_version {
		Some(ver) => get_base_version(ver),
		None => get_latest_base_version(sc2_path.as_str()),
	};
	let sc2_binary = if cfg!(target_os = "windows") {
		"SC2_x64.exe"
	} else if cfg!(target_os = "linux") {
		"SC2_x64"
	} else {
		panic!("Unsupported OS")
	};

	print!("Launching SC2 process... ");
	flush();
	let mut process = Command::new(format!(
		"{}/Versions/Base{}/{}",
		sc2_path, base_version, sc2_binary
	))
	.current_dir(format!("{}/Support64", sc2_path))
	.arg("-listen")
	.arg(host)
	.arg("-port")
	.arg(port.to_string())
	// 0 - windowed, 1 - fullscreen
	.arg("-displayMode")
	.arg("0")
	/*
	.arg("-windowX")
	.arg("10")
	.arg("-windowY")
	.arg("10")
	.arg("-windowWidth")
	.arg("1024")
	.arg("-windowHeight")
	.arg("768")
	*/
	.spawn()
	.expect("Can't launch SC2 process.");
	println!("Done");

	print!("Connecting to websocket... ");
	flush();
	let url = Url::parse(format!("ws://{}:{}/sc2api", host, port).as_str()).unwrap();
	let (mut ws, _rs) = loop {
		if let Ok(result) = connect(url.clone()) {
			break result;
		}
	};
	println!("Done");

	match {
		print!("Sending CreateGame request... ");
		flush();
		// Create game
		let mut req = Request::new();
		let req_create_game = req.mut_create_game();
		req_create_game
			.mut_local_map()
			.set_map_path(format!("{}/Maps/{}.SC2Map", sc2_path, map_name));
		/*
		Set Map
			req.mut_create_game().mut_local_map().set_map_path("");
		OR
			req.mut_create_game().set_battlenet_map_name("");
		*/
		create_player_setup(&player, req_create_game);
		create_computer_setup(&computer, req_create_game);
		// req_create_game.set_disable_fog(bool); // Cheat
		// req_create_game.set_random_seed(u32);
		req_create_game.set_realtime(realtime);

		let res = send(&mut ws, req)?;
		let res_create_game = res.get_create_game();
		if res_create_game.has_error() {
			panic!(
				"{:?}: {}",
				res_create_game.get_error(),
				res_create_game.get_error_details()
			);
		}
		println!("Done");

		print!("Sending JoinGame request... ");
		flush();
		join_game(player, &mut ws, None)?;
		println!("Done");

		print!("Requesting GameInfo... ");
		flush();
		let mut req = Request::new();
		req.mut_game_info();
		let res = send(&mut ws, req)?;
		player.set_game_info(GameInfo::from_proto(res.get_game_info().clone()));
		println!("Done");

		print!("Requesting GameData... ");
		flush();
		let mut req = Request::new();
		let req_game_data = req.mut_data();
		req_game_data.set_ability_id(true);
		req_game_data.set_unit_type_id(true);
		req_game_data.set_upgrade_id(true);
		req_game_data.set_buff_id(true);
		req_game_data.set_effect_id(true);

		let res = send(&mut ws, req)?;
		player.set_game_data(GameData::from_proto(res.get_data().clone()));
		println!("Done");

		print!("Entered main loop... ");
		flush();
		let is_human = player.get_player_settings().player_type == PlayerType::Human;
		// Main loop
		play_first_step(&mut ws, player, realtime, is_human)?;
		let mut iteration = 0;
		loop {
			if !play_step(&mut ws, player, iteration, realtime, is_human)? {
				break;
			}
			iteration += 1;
		}
		println!("Finished");
		Ok(())
	} {
		Ok(()) => terminate(&mut process, &mut ws),
		Err(why) => {
			terminate(&mut process, &mut ws)?;
			Err(why)
		}
	}
}

fn launch_pvp(
	player1: &mut PlayerBox,
	player2: &mut PlayerBox,
	map_name: String,
	realtime: bool,
	sc2_version: Option<String>,
) -> TResult<()> {
	println!("Starting PvP game.");
	// config

	let ports = get_unused_ports(9);
	let (host, port_host, port_client): (&str, i32, i32) = ("127.0.0.1", ports[0], ports[1]);
	let sc2_path = get_path_to_sc2();
	let base_version = match sc2_version {
		Some(ver) => get_base_version(ver),
		None => get_latest_base_version(sc2_path.as_str()),
	};
	let sc2_binary = if cfg!(target_os = "windows") {
		"SC2_x64.exe"
	} else if cfg!(target_os = "linux") {
		"SC2_x64"
	} else {
		panic!("Unsupported OS")
	};

	print!("Launching SC2 processes... ");
	flush();
	let mut process_host = Command::new(format!(
		"{}/Versions/Base{}/{}",
		sc2_path, base_version, sc2_binary
	))
	.current_dir(format!("{}/Support64", sc2_path))
	.arg("-listen")
	.arg(host)
	.arg("-port")
	.arg(port_host.to_string())
	.arg("-displayMode")
	.arg("0")
	.spawn()
	.expect("Can't launch 1st SC2 process.");
	let mut process_client = Command::new(format!(
		"{}/Versions/Base{}/{}",
		sc2_path, base_version, sc2_binary
	))
	.current_dir(format!("{}/Support64", sc2_path))
	.arg("-listen")
	.arg(host)
	.arg("-port")
	.arg(port_client.to_string())
	.arg("-displayMode")
	.arg("0")
	.spawn()
	.expect("Can't launch 2nd SC2 process.");
	println!("Done");

	print!("Connecting to websockets... ");
	flush();
	let url = Url::parse(format!("ws://{}:{}/sc2api", host, port_host).as_str()).unwrap();
	let (mut ws_host, _rs) = loop {
		if let Ok(result) = connect(url.clone()) {
			break result;
		}
	};
	let url = Url::parse(format!("ws://{}:{}/sc2api", host, port_client).as_str()).unwrap();
	let (mut ws_client, _rs) = loop {
		if let Ok(result) = connect(url.clone()) {
			break result;
		}
	};
	println!("Done");

	match {
		print!("Sending CreateGame request to host process... ");
		flush();
		let mut req = Request::new();
		let req_create_game = req.mut_create_game();
		req_create_game
			.mut_local_map()
			.set_map_path(format!("{}/Maps/{}.SC2Map", sc2_path, map_name));
		create_player_setup(&player1, req_create_game);
		create_player_setup(&player2, req_create_game);
		// req_create_game.set_disable_fog(bool); // Cheat
		// req_create_game.set_random_seed(u32);
		req_create_game.set_realtime(realtime);

		let res = send(&mut ws_host, req)?;
		let res_create_game = res.get_create_game();
		if res_create_game.has_error() {
			panic!(
				"{:?}: {}",
				res_create_game.get_error(),
				res_create_game.get_error_details()
			);
		}
		println!("Done");

		print!("Sending JoinGame request to both processes... ");
		flush();
		let ports = Ports {
			shared: ports[2],
			server: [ports[3], ports[4]],
			client: vec![[ports[5], ports[6]], [ports[7], ports[8]]],
		};
		join_game(player1, &mut ws_host, Some(ports.clone()))?;
		join_game(player2, &mut ws_client, Some(ports))?;
		println!("Done");

		print!("Requesting GameInfo... ");
		flush();
		let mut req = Request::new();
		req.mut_game_info();
		player1.set_game_info(GameInfo::from_proto(
			send(&mut ws_host, req.clone())?.get_game_info().clone(),
		));
		player2.set_game_info(GameInfo::from_proto(
			send(&mut ws_client, req)?.get_game_info().clone(),
		));
		println!("Done");

		print!("Requesting GameData... ");
		flush();
		let mut req = Request::new();
		let req_game_data = req.mut_data();
		req_game_data.set_ability_id(true);
		req_game_data.set_unit_type_id(true);
		req_game_data.set_upgrade_id(true);
		req_game_data.set_buff_id(true);
		req_game_data.set_effect_id(true);

		player1.set_game_data(GameData::from_proto(
			send(&mut ws_host, req.clone())?.get_data().clone(),
		));
		player2.set_game_data(GameData::from_proto(
			send(&mut ws_client, req)?.get_data().clone(),
		));
		println!("Done");

		print!("Entered main loop... ");
		flush();
		let is_human1 = player1.get_player_settings().player_type == PlayerType::Human;
		let is_human2 = player2.get_player_settings().player_type == PlayerType::Human;
		play_first_step(&mut ws_host, player1, realtime, is_human1)?;
		play_first_step(&mut ws_client, player2, realtime, is_human2)?;
		let mut iteration = 0;
		loop {
			if !play_step(&mut ws_host, player1, iteration, realtime, is_human1)?
				|| !play_step(&mut ws_client, player2, iteration, realtime, is_human2)?
			{
				break;
			}
			iteration += 1;
		}
		println!("Finished");
		Ok(())
	} {
		Ok(()) => terminate_both(
			&mut process_host,
			&mut ws_host,
			&mut process_client,
			&mut ws_client,
		),
		Err(why) => {
			terminate_both(
				&mut process_host,
				&mut ws_host,
				&mut process_client,
				&mut ws_client,
			)?;
			Err(why)
		}
	}
}
