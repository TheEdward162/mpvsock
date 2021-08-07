use std::io::{self, Read};

pub struct ResponseBuffer {
	buffer: Vec<u8>,
	position: usize
}
impl ResponseBuffer {
	const LINE_DELIM: u8 = b'\n';
	const RESERVE_SIZE: usize = 128;

	pub fn new() -> Self {
		ResponseBuffer {
			buffer: Vec::with_capacity(Self::RESERVE_SIZE),
			position: 0
		}
	}

	pub fn read_nonblocking(&mut self, mut stream: impl Read) -> Result<(), io::Error> {
		match stream.read_to_end(&mut self.buffer) {
			Ok(_) => (),
			Err(err) if err.kind() == io::ErrorKind::WouldBlock => (),
			Err(err) => return Err(err)
		}

		Ok(())
	}

	pub fn read_blocking(&mut self, stream: impl Read) -> Result<(), io::Error> {
		for byte in stream.bytes() {
			let byte = match byte {
				Ok(byte) => byte,
				Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
				Err(err) => return Err(err)
			};

			if byte == Self::LINE_DELIM {
				break
			}

			self.buffer.push(byte);
		}

		Ok(())
	}

	pub fn read_from(&mut self, mut stream: impl Read) -> Result<(), io::Error> {
		if self.buffer.len() + Self::RESERVE_SIZE >= self.buffer.capacity() {
			self.buffer
				.resize(self.buffer.len() + Self::RESERVE_SIZE, 0);
		}

		match stream.read(&mut self.buffer) {
			Ok(_) => (),
			Err(err) if err.kind() == io::ErrorKind::WouldBlock => (),
			Err(err) => return Err(err)
		}

		Ok(())
	}

	pub fn consume_line(&mut self) -> Option<&[u8]> {
		let next_newline = self.buffer[self.position ..]
			.iter()
			.position(|&b| b == Self::LINE_DELIM);

		match next_newline {
			None => None,
			Some(end) => {
				let line = &self.buffer[self.position ..][.. end];
				self.position += end + 1;

				if log::log_enabled!(log::Level::Debug) {
					match std::str::from_utf8(line) {
						Ok(line) => {
							log::trace!("Consumed line: {}", line);
						}
						Err(_err) => {
							log::trace!("Consumed line: {:?}", line)
						}
					}
				}

				Some(line)
			}
		}
	}

	pub fn shift(&mut self) {
		log::trace!("Shifting buffer by {} elements", self.position);

		self.buffer.drain(.. self.position);
		self.position = 0;
	}
}
