use std::{
	fmt::Write as FmtWrite,
	io::{self, BufRead, Write},
	path::Path
};

use clap::{App, Arg, ArgGroup, ArgMatches, SubCommand};

use mpvsock::link::MpvLink;

fn parse_cli() -> ArgMatches<'static> {
	App::new(env!("CARGO_PKG_NAME"))
		.version(env!("CARGO_PKG_VERSION"))
		.arg(
			Arg::with_name("verbosity")
				.short("v")
				.long("verbosity")
				.takes_value(true)
				.default_value("Off")
				.possible_values(&["Off", "Error", "Warn", "Info", "Debug", "Trace"])
				.help("Level of verbosity")
		)
		// spawn options
		.arg(
			Arg::with_name("connect")
				.long("connect")
				.takes_value(true)
				.value_name("socket_path")
				.help("Connect to an existing mpv socket")
		)
		.arg(
			Arg::with_name("spawn_server")
				.long("spawn-server")
				.takes_value(true)
				.value_name("socket_path")
				.help("Spawn a new mpv process that acts as a server opening a socket at given path")
		)
		.arg(
			Arg::with_name("spawn_client")
				.long("spawn-client")
				.takes_value(false)
				.help("Spawn a new mpv process that acts as a client listening on an unnamed socket")
		)
		.group(
			ArgGroup::with_name("mpv_link")
				.args(&["connect", "spawn_server", "spawn_client"])
				.required(true)
		)
		// interactive subcommand
		.subcommand(
			SubCommand::with_name("interactive")
				.about("Opens and interactive command prompt")
				.arg(
					Arg::with_name("raw")
						.long("raw")
						.takes_value(false)
						.help("Send input lines raw as the command array. The line must be valid JSON array without the `[]` enclosing characters.")
				)
		)
		.get_matches()
}

fn setup_logger(level: log::Level) {
	edwardium_logger::Logger::new(
		edwardium_logger::targets::stderr::StderrTarget::new(level, Default::default()),
		std::time::Instant::now()
	)
	.init_boxed()
	.expect("Could not initialize logger");
}

fn main() -> anyhow::Result<()> {
	let matches = parse_cli();

	if let Some(level) = match matches.value_of("verbosity").unwrap() {
		"Off" => None,
		"Error" => Some(log::Level::Error),
		"Warn" => Some(log::Level::Warn),
		"Info" => Some(log::Level::Info),
		"Debug" => Some(log::Level::Debug),
		"Trace" => Some(log::Level::Trace),
		_ => unreachable!()
	} {
		setup_logger(level);
		log::debug!("{:?}", matches);
	}

	let mut mpv = if let Some(socket_path) = matches.value_of("connect") {
		MpvLink::connect(Path::new(socket_path))?
	} else if let Some(socket_path) = matches.value_of("spawn_server") {
		MpvLink::spawn_server(Path::new(socket_path))?
	} else if matches.is_present("spawn_client") {
		MpvLink::spawn_client()?
	} else {
		unreachable!()
	};

	if let Some(matches) = matches.subcommand_matches("interactive") {
		let mut context = InteractiveContext::new(&matches);
		context.run(&mut mpv)?;
	}

	Ok(())
}

struct InteractiveContext {
	line: String,
	command: String,
	is_raw: bool
}
impl InteractiveContext {
	pub fn new(matches: &ArgMatches) -> Self {
		InteractiveContext {
			line: String::new(),
			command: String::new(),
			is_raw: matches.is_present("raw")
		}
	}

	pub fn run(&mut self, mpv: &mut MpvLink) -> anyhow::Result<()> {
		let stdin = io::stdin();
		let stdout = io::stdout();
		let mut stdin = stdin.lock();
		let mut stdout = stdout.lock();

		self.write_help(&mut stdout)?;

		loop {
			write!(stdout, "Input: ")?;
			stdout.flush()?;

			self.line.clear();
			match stdin.read_line(&mut self.line)? {
				0 => break,
				_ => ()
			};
			if self.line.ends_with('\n') {
				self.line.pop();
			}

			if self.line.starts_with("#") {
				match self.handle_input_command(&mut stdout, mpv)? {
					true => break,
					false => continue
				}
			}

			let cmd = if self.is_raw {
				&self.line
			} else {
				self.build_command()?
			};

			match mpv.run_command(cmd) {
				Ok(result) => {
					writeln!(&mut stdout, "Result: {:?}", result)?;
				}
				Err(err) => {
					writeln!(&mut stdout, "Error: {}", err)?;
				}
			};
		}

		Ok(())
	}

	fn handle_input_command(
		&mut self,
		mut out: impl Write,
		mpv: &mut MpvLink
	) -> anyhow::Result<bool> {
		let res = match self.line.as_str() {
			"#events" => {
				let events = mpv.poll_events()?;
				writeln!(&mut out, "Events ({}):", events.len())?;
				for event in events {
					writeln!(&mut out, "\t{:?}", event)?;
				}

				mpv.clear_events();

				false
			}
			"#raw" => {
				self.is_raw = !self.is_raw;
				self.write_raw_mode(&mut out)?;

				false
			}
			"#quit" => true,
			"#help" => {
				self.write_help(&mut out)?;

				false
			}
			_ => {
				writeln!(&mut out, "Error: Invalid input command")?;

				false
			}
		};

		Ok(res)
	}

	fn write_help(&self, mut out: impl Write) -> Result<(), io::Error> {
		writeln!(&mut out, "Help:")?;
		writeln!(&mut out, "\tInput commands: #help #events #raw #quit")?;

		self.write_raw_mode(&mut out)?;

		writeln!(&mut out)?;

		Ok(())
	}

	fn write_raw_mode(&self, mut out: impl Write) -> Result<(), io::Error> {
		if self.is_raw {
			writeln!(&mut out, "\tRaw mode is on")?;
		} else {
			writeln!(
				&mut out,
				"\tRaw mode is off, prefix values with @ to pass them verbatim"
			)?;
		}

		Ok(())
	}

	fn build_command(&mut self) -> Result<&str, std::fmt::Error> {
		self.command.clear();

		for word in self.line.split(' ') {
			if word.starts_with("@@") {
				write!(&mut self.command, "\"{}\",", &word[1 ..])?;
			} else if word.starts_with("@") {
				write!(&mut self.command, "{},", &word[1 ..])?;
			} else {
				write!(&mut self.command, "\"{}\",", word)?;
			}
		}

		// remove the trailing comma
		Ok(&self.command[.. self.command.len().saturating_sub(1)])
	}
}
