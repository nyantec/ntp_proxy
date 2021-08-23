use std::net::Ipv4Addr;

use afpacket::r#async::RawPacketStream;
use anyhow::{Context, Result};
use async_std::prelude::*;
use log::*;
use pnet::packet::ethernet::{EtherTypes, MutableEthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::MutableIpv4Packet;
use pnet::packet::udp::{ipv4_checksum_adv, MutableUdpPacket};
use pnet::packet::{MutablePacket, Packet, PacketSize};

pub fn setup_interface(interface: &str, dst_port: u16) -> Result<RawPacketStream> {
	let mut stream = RawPacketStream::new().context("Open interface")?;

	stream
		.bind(interface)
		.with_context(|| format!("Bind to interface {}", interface))?;

	// tcpdump -p -ni lo -ddd "udp and dst port ${dst_port}"
	stream
		.set_bpf_filter(vec![
			(40, 0, 0, 12),
			(21, 0, 4, 34525),
			(48, 0, 0, 20),
			(21, 0, 11, 17),
			(40, 0, 0, 56),
			(21, 8, 9, 123),
			(21, 0, 8, 2048),
			(48, 0, 0, 23),
			(21, 0, 6, 17),
			(40, 0, 0, 20),
			(69, 4, 0, 8191),
			(177, 0, 0, 14),
			(72, 0, 0, 16),
			(21, 0, 1, dst_port as u32),
			(6, 0, 0, 262144),
			(6, 0, 0, 0),
		])
		.context("Set bpf filter to interface")?;

	info!("Opened source network");
	Ok(stream)
}

pub fn setup_outerface(interface: &str) -> Result<RawPacketStream> {
	let mut stream = RawPacketStream::new().context("Open interface")?;
	stream
		.bind(interface)
		.with_context(|| format!("Bind to interface {}", interface))?;

	info!("opened destination network");
	Ok(stream)
}

pub async fn run(
	mut in_stream: RawPacketStream,
	mut out_stream: RawPacketStream,
	rewrite_address: Ipv4Addr,
	rewrite_port: Option<u16>,
	dst_port: u16,
	rewrite_dst_addr: Option<Ipv4Addr>,
) -> Result<()> {
	info!("starting run loop");
	loop {
		// FIXME: only allocate once, and write 0es in it at each loop
		let mut buf = [0u8; 1500];
		in_stream
			.read(&mut buf)
			.await
			.context("Failed to read input stream")?;

		debug!("got packet");

		let mut ethernet = MutableEthernetPacket::new(&mut buf).context("Parsing EthernetFrame")?;
		if ethernet.get_ethertype() != EtherTypes::Ipv4 {
			trace!("invalid ethertype: {}", ethernet.get_ethertype());
			continue;
		}

		let length = ethernet.packet_size();

		let mut ipv4 =
			MutableIpv4Packet::new(ethernet.payload_mut()).context("Parsing ipv4 packet")?;

		if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Udp {
			trace!(
				"invalid next level protocol: {}",
				ipv4.get_next_level_protocol()
			);
			continue;
		}

		trace!(
			"rewriting source address to {}, from {}",
			rewrite_address,
			ipv4.get_source()
		);
		ipv4.set_source(rewrite_address);
		if let Some(addr) = rewrite_dst_addr {
			trace!(
				"rewriting destination address from {} to {}",
				ipv4.get_destination(),
				addr
			);
			ipv4.set_destination(addr);
		}

		ipv4.set_checksum(pnet::packet::ipv4::checksum(&ipv4.to_immutable()));

		let ipv4_source = ipv4.get_source();
		let ipv4_dest = ipv4.get_destination();
		let length = length + ipv4.packet_size();

		let mut udp = MutableUdpPacket::new(ipv4.payload_mut()).context("Parsing udp packet")?;
		if udp.get_destination() != dst_port {
			trace!("invalid udp port: {}", udp.get_destination());
			continue;
		}

		if let Some(port) = rewrite_port {
			trace!("rewriting source port to {}", port);
			udp.set_source(port);
		}

		let checksum = ipv4_checksum_adv(&udp.to_immutable(), &[], &ipv4_source, &ipv4_dest);
		udp.set_checksum(checksum);

		let length = length + udp.get_length() as usize;

		debug!("writing packet: {:X?}", &ethernet.packet()[..length]);
		out_stream
			.write_all(&ethernet.packet()[..length])
			.await
			.context("Writing packet")?;
	}
}
