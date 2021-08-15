use std::{
	fmt::Write as FmtWrite,
	io::{self, BufRead, Write},
	path::Path
};

use anyhow::Context;
use clap::{App, Arg, ArgGroup, ArgMatches, SubCommand};

use mpvsock::{command::commands::{CmdCycleProperty, CmdGetProperty, CmdGetVersion, CmdSeek, CmdSetProperty}, link::MpvLink};

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

enum InputMode {
	Raw,
	String,
	Known
}

struct InteractiveContext {
	line: String,
	command: String,
	mode: InputMode
}
macro_rules! write_result_and_bail {
	(
		$out: expr; $result: expr
	) => {
		match $result {
			Ok(result) => {
				writeln!($out, "Result: {:?}", result)?;

				return Ok(())
			}
			Err(err) => {
				writeln!($out, "Error: {}", err)?;

				return Ok(())
			}
		}
	};
}
macro_rules! write_error_and_bail {
	(
		$out: expr; $result: expr
	) => {
		match $result {
			Ok(result) => result,
			Err(err) => {
				writeln!($out, "Error: {}", err)?;

				return Ok(())
			}
		}
	};
}
impl InteractiveContext {
	pub fn new(_matches: &ArgMatches) -> Self {
		InteractiveContext {
			line: String::new(),
			command: String::new(),
			mode: InputMode::String
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

			match self.mode {
				InputMode::Raw => self.run_raw_command(mpv, &mut stdout),
				InputMode::String => self.run_string_command(mpv, &mut stdout),
				InputMode::Known => self.run_known_command(mpv, &mut stdout)
			}?;
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
				mpv.poll_events()?;

				let events = mpv.drain_events();
				writeln!(&mut out, "Events ({}):", {
					let hint = events.size_hint();
					match hint.1 {
						None => hint.0,
						Some(hint) => hint
					}
				})?;
				for event in events {
					writeln!(&mut out, "\t{:?}", event)?;
				}

				false
			}
			"#mode raw" => {
				self.mode = InputMode::Raw;
				self.write_mode(&mut out)?;

				false
			}
			"#mode string" => {
				self.mode = InputMode::String;
				self.write_mode(&mut out)?;

				false
			}
			"#mode known" => {
				self.mode = InputMode::Known;
				self.write_mode(&mut out)?;

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
		writeln!(
			&mut out,
			"\tInput commands:\n\t\t#help\n\t\t#events\n\t\t#mode raw|string|known\n\t\t#quit"
		)?;

		self.write_mode(&mut out)?;

		writeln!(&mut out)?;

		Ok(())
	}

	fn write_mode(&self, mut out: impl Write) -> Result<(), io::Error> {
		match self.mode {
			InputMode::Raw => {
				writeln!(
					&mut out,
					"\tRaw mode is on, input is directly pasted as JSON array elements"
				)?;
			}
			InputMode::String => {
				writeln!(&mut out, "\tString mode is on, input is split by spaces and elements are quoted (prefix element with @ to disable quoting)")?;
			}
			InputMode::Known => {
				writeln!(&mut out, "\tKnown mode is on, only known commands are accepted and their result is properly parsed")?;
				writeln!(
					&mut out,
					"\tKnown commands: get_version get_property set_property cycle seek"
				)?;
			}
		}

		Ok(())
	}

	fn run_raw_command(&mut self, mpv: &mut MpvLink, mut out: impl Write) -> anyhow::Result<()> {
		write_result_and_bail!(out; mpv.run_command(self.line.as_str()))
	}

	fn run_string_command(&mut self, mpv: &mut MpvLink, mut out: impl Write) -> anyhow::Result<()> {
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
		let command = &self.command[.. self.command.len().saturating_sub(1)];

		write_result_and_bail!(out; mpv.run_command(command))
	}

	fn run_known_command(&mut self, mpv: &mut MpvLink, mut out: impl Write) -> anyhow::Result<()> {
		use mpvsock::command::property;

		if self.line.trim() == "get_version" {
			write_result_and_bail!(out; mpv.run_command(&CmdGetVersion))
		}

		if self.line.starts_with("get_property ") || self.line.starts_with("get ") {
			let mut iter = self.line.splitn(2, ' ');
			iter.next().unwrap(); // get_property
			let property_name = write_error_and_bail!(
				&mut out; iter.next().context("get_property expects an argument")
			);

			macro_rules! choose_property {
				(
					$(
						$known_struct: ident: $known_name: literal
					),+ $(,)?
				) => {
					match property_name {
						$(
							$known_name => {
								let command = CmdGetProperty::new(property::$known_struct);
								write_result_and_bail!(out; mpv.run_command(&command))
							}
						)+
						_ => {
							let command = CmdGetProperty::new(property_name);
							write_result_and_bail!(out; mpv.run_command(&command))
						}
					}
				}
			}

			choose_property!(
				Volume: "volume",
				PercentPos: "percent-pos",
				TimePos: "time-pos",
				Path: "path",
				WorkingDirectory: "working-directory",
				MediaTitle: "media-title",
				Aid: "aid",
				Vid: "vid",
				Sid: "sid",
				Fullscreen: "fullscreen",
				Pause: "pause",
			)
		}

		if self.line.starts_with("set_property ") || self.line.starts_with("set ") {
			let mut iter = self.line.splitn(3, ' ');
			iter.next().unwrap(); // set_property
			let property_name = write_error_and_bail!(
				&mut out; iter.next().context("set_property expects two arguments")
			);
			let property_value = write_error_and_bail!(
				&mut out; iter.next().context("set_property expects two arguments")
			);

			macro_rules! choose_property {
				(
					$(
						$known_struct: ident: $known_name: literal
					),+ $(,)?
				) => {
					match property_name {
						$(
							$known_name => {
								let command = CmdSetProperty::new(
									property::$known_struct,
									serde_json::from_str(property_value)?
								);
								write_result_and_bail!(out; mpv.run_command(&command))
							}
						)+
						_ => {
							let command = CmdSetProperty::new(property_name, property_value.into());
							write_result_and_bail!(out; mpv.run_command(&command))
						}
					}
				}
			}

			choose_property!(
				Volume: "volume",
				PercentPos: "percent-pos",
				TimePos: "time-pos",
				Path: "path",
				WorkingDirectory: "working-directory",
				MediaTitle: "media-title",
				Aid: "aid",
				Vid: "vid",
				Sid: "sid",
				Fullscreen: "fullscreen",
				Pause: "pause",
			)
		}

		if self.line.starts_with("cycle ") {
			let mut iter = self.line.splitn(2, ' ');
			iter.next().unwrap(); // cycle
			let property_name = write_error_and_bail!(
				&mut out; iter.next().context("cycle expects an argument")
			);

			macro_rules! choose_property {
				(
					$(
						$known_struct: ident: $known_name: literal
					),+ $(,)?
				) => {
					match property_name {
						$(
							$known_name => {
								let command = CmdCycleProperty::new(property::$known_struct, false);
								write_result_and_bail!(out; mpv.run_command(&command))
							}
						)+
						_ => {
							let command = CmdCycleProperty::new(property_name, false);
							write_result_and_bail!(out; mpv.run_command(&command))
						}
					}
				}
			}

			choose_property!(
				Volume: "volume",
				PercentPos: "percent-pos",
				TimePos: "time-pos",
				Path: "path",
				WorkingDirectory: "working-directory",
				MediaTitle: "media-title",
				Aid: "aid",
				Vid: "vid",
				Sid: "sid",
				Fullscreen: "fullscreen",
				Pause: "pause",
			)
		}

		if self.line.starts_with("seek ") {
			let mut iter = self.line.splitn(3, ' ');
			iter.next().unwrap(); // seek
			let target = write_error_and_bail!(
				&mut out; iter.next().context("seek expects at least one argument").and_then(|value| value.parse::<f64>().context("seek expects an f64 first argument"))
			);
			
			let command = match iter.next() {
				Some("absolute") => CmdSeek::time(target, true),
				Some("absolute-percent") => CmdSeek::percent(target, true),
				Some("relative-percent") => CmdSeek::percent(target, true),
				_ => CmdSeek::time(target, false)
			};

			write_result_and_bail!(out; mpv.run_command(&command))
		}

		writeln!(out, "Unrecognized command")?;
		Ok(())
	}
}
