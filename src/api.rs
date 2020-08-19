//! Contains nice wrapper around SC2 API.

use crate::client::{SC2Result, WS};
use protobuf::Message;
use sc2_proto::sc2api::{Request, Response};
use tungstenite::Message::Binary;

/// SC2 API. Can be accessed through [`self.api()`](crate::bot::Bot::api).
pub struct API(WS);
impl API {
	/// Sends request and returns a response.
	pub fn send(&mut self, req: Request) -> SC2Result<Response> {
		self.0.write_message(Binary(req.write_to_bytes()?))?;

		let msg = self.0.read_message()?;

		let mut res = Response::new();
		res.merge_from_bytes(msg.into_data().as_slice())?;
		Ok(res)
	}

	/// Sends request, waits for the response, but ignores it (useful when response is empty).
	pub fn send_request(&mut self, req: Request) -> SC2Result<()> {
		self.0.write_message(Binary(req.write_to_bytes()?))?;
		let _ = self.0.read_message()?;
		Ok(())
	}

	/// Sends request, but doesn't wait for the response (use only when more control required,
	/// in common cases prefered to use [`send`] or [`send_request`]).
	///
	/// [`send`]: Self::send
	/// [`send_request`]: Self::send_request
	pub fn send_only(&mut self, req: Request) -> SC2Result<()> {
		self.0.write_message(Binary(req.write_to_bytes()?))?;
		Ok(())
	}
	/// Waits for a response (useful only after [`send_only`]).
	///
	/// [`send_only`]: Self::send_only
	pub fn wait_response(&mut self) -> SC2Result<Response> {
		let msg = self.0.read_message()?;

		let mut res = Response::new();
		res.merge_from_bytes(msg.into_data().as_slice())?;
		Ok(res)
	}
}
