use std::net::Ipv4Addr;

use nix::net::if_::if_nametoindex;

pub fn is_interface(interface: String) -> Result<(), String> {
	if_nametoindex(interface.as_str()).map_err(|e| format!("{}: {}", interface, e))?;
	Ok(())
}

pub fn is_address4(addr: String) -> Result<(), String> {
	addr.parse::<Ipv4Addr>()
		.map_err(|e| format!("{}: {}", addr, e))?;
	Ok(())
}

pub fn is_port(port: String) -> Result<(), String> {
	port.parse::<u16>()
		.map_err(|e| format!("{}: {}", port, e))?;
	Ok(())
}
