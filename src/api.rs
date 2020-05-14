use crate::client::{SC2Result, WS};
use protobuf::Message;
use sc2_proto::sc2api::{Request, Response};
use tungstenite::Message::Binary;

pub struct API(pub WS);
impl API {
	pub fn send_request(&mut self, req: Request) -> SC2Result<()> {
		self.0.write_message(Binary(req.write_to_bytes()?))?;
		let _ = self.0.read_message()?;
		Ok(())
	}

	pub fn send(&mut self, req: Request) -> SC2Result<Response> {
		self.0.write_message(Binary(req.write_to_bytes()?))?;

		let msg = self.0.read_message()?;

		let mut res = Response::new();
		res.merge_from_bytes(msg.into_data().as_slice())?;
		Ok(res)
	}

	pub fn send_only(&mut self, req: Request) -> SC2Result<()> {
		self.0.write_message(Binary(req.write_to_bytes()?))?;
		Ok(())
	}
	pub fn wait_response(&mut self) -> SC2Result<Response> {
		let msg = self.0.read_message()?;

		let mut res = Response::new();
		res.merge_from_bytes(msg.into_data().as_slice())?;
		Ok(res)
	}
}
