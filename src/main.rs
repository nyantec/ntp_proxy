mod proxy;
mod validators;

use anyhow::Result;
use clap::{App, Arg};

#[async_std::main]
async fn main() {
	if let Err(e) = main_err().await {
		eprintln!("Error:");
		eprintln!("{:?}", e);
		std::process::exit(1);
	}
}

async fn main_err() -> Result<()> {
	env_logger::init();
	let app = App::new("ntp_proxy")
		.version(env!("CARGO_PKG_VERSION"))
		.author(env!("CARGO_PKG_AUTHORS"))
		.about("Forwarder for ntp broadcast traffic")
		.setting(clap::AppSettings::ColorAuto)
		.setting(clap::AppSettings::ColoredHelp)
		.arg(
			Arg::with_name("interface")
				.long("interface")
				.short("i")
				.value_name("INTERFACE")
				.help("inner interface to listen for ntp packages to forward")
				.takes_value(true)
				.env("NTPPROXY_INTERFACE")
				.default_value("lo")
				.required(true)
				.validator(validators::is_interface),
		)
		.arg(
			Arg::with_name("outerface")
				.long("outerface")
				.short("o")
				.value_name("INTERFACE")
				.help("outer interface to forward ntp package to")
				.takes_value(true)
				.env("NTPPROXY_OUTERFACE")
				.required(true)
				.validator(validators::is_interface),
		)
		.arg(
			Arg::with_name("address")
				.long("address")
				.short("a")
				.value_name("ADDRESS")
				.help("address to write into ipv4 header")
				.env("NTPPROXY_ADDRESS")
				.required(true)
				.validator(validators::is_address4),
		)
		.arg(
			Arg::with_name("port")
				.long("port")
				.short("p")
				.value_name("PORT")
				.help("port to set as source port in upd header")
				//.default_value("0")
				.env("NTPPROXY_PORT")
				.validator(validators::is_port),
		)
		.arg(
			Arg::with_name("dst_port")
				.long("dst-port")
				.value_name("PORT")
				.help("Port to listen for ntp packages")
				.default_value("123")
				.env("NTPPROXY_DST_PORT")
				.required(true)
				.validator(validators::is_port),
		)
		.arg(
			Arg::with_name("dst_addr")
				.long("dst-addr")
				.value_name("ADDRESS")
				.help("new destination address for packets")
				.env("NTPPROXY_DST_ADDR")
				.required(false)
				.validator(validators::is_address4),
		);

	let matches = app.get_matches();

	let in_stream = proxy::setup_interface(
		// SAFETY: all is validate before
		matches.value_of("interface").unwrap(),
		// SAFETY: all is validate before
		matches.value_of("dst_port").unwrap().parse().unwrap(),
	)?;

	let out_stream = proxy::setup_outerface(
		// SAFETY: all is validate before
		matches.value_of("outerface").unwrap(),
	)?;

	proxy::run(
		in_stream,
		out_stream,
		// SAFETY: address is validated by clap
		matches.value_of("address").unwrap().parse().unwrap(),
		matches.value_of("port").map(|p| p.parse().ok()).flatten(),
		// SAFETY: dst_port is validated by clap
		matches.value_of("dst_port").unwrap().parse().unwrap(),
		matches
			.value_of("dst_addr")
			.map(|a| a.parse().ok())
			.flatten(),
	)
	.await?;
	Ok(())
}
