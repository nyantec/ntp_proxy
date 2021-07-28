use afpacket::sync::RawPacketStream;
use anyhow::{Context, Result};
use pnet::packet::ethernet::{EtherTypes, Ethernet, EthernetPacket, MutableEthernetPacket};
use pnet::packet::icmp::echo_request::{EchoRequest, IcmpCodes, MutableEchoRequestPacket};
use pnet::packet::icmp::{IcmpPacket, IcmpTypes};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4;
use pnet::packet::ipv4::{Ipv4, Ipv4Packet, MutableIpv4Packet};
use pnet::packet::udp::MutableUdpPacket;
use pnet::packet::Packet;
use pnet::util::checksum;
use pnet::util::MacAddr;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::Ipv4Addr;
use std::ptr::eq;

fn main() {
    let mut ps = RawPacketStream::new().unwrap();
    ps.bind("lo").unwrap();

    // tcpdump -p -ni lo -ddd "udp"
    ps.set_bpf_filter(vec![
        // length: 12
        (40, 0, 0, 12),
        (21, 0, 5, 34525),
        (48, 0, 0, 20),
        (21, 6, 0, 17),
        (21, 0, 6, 44),
        (48, 0, 0, 54),
        (21, 3, 4, 17),
        (21, 0, 3, 2048),
        (48, 0, 0, 23),
        (21, 0, 1, 17),
        (6, 0, 0, 262144),
        (6, 0, 0, 0),
    ])
    .unwrap();

    let mut out = RawPacketStream::new().unwrap();
    out.bind("veth0").unwrap();

    ntp_pnet(ps, out).unwrap();
}

fn ntp_pnet(mut ps: RawPacketStream, mut out: RawPacketStream) -> Result<()> {
    let mut offset;
    loop {
        let buf = RefCell::new([0u8; 1500]);
        ps.read(&mut *buf.borrow_mut());
        /*{
            let mut buf = *buf.borrow_mut();
            buf = [0; 1500];
            ps.read(&mut buf)?;
        }*/

        {
            let buf = *buf.borrow();
            let ethernet = EthernetPacket::new(&buf).context("ethernet")?;
            //println!("{:?}", ethernet);

            if ethernet.get_ethertype() != EtherTypes::Ipv4 {
                continue;
            }
            offset = EthernetPacket::minimum_packet_size();
        }

        let (ipv4_source, ipv4_dest, length) = {
            let mut buf = *buf.borrow_mut();
            let mut ipv4 = MutableIpv4Packet::new(&mut buf[offset..]).context("ipv4")?;
            //println!("{:?}", ipv4);
            if ipv4.get_next_level_protocol() != IpNextHeaderProtocols::Udp {
                continue;
            }

            ipv4.set_source(Ipv4Addr::new(172, 16, 16, 1));
            ipv4.set_checksum(pnet::packet::ipv4::checksum(&ipv4.to_immutable()));

            offset += (ipv4.get_header_length() * 4) as usize;
            (
                ipv4.get_source(),
                ipv4.get_destination(),
                ipv4.get_total_length() as usize - (ipv4.get_header_length() * 4) as usize,
            )
        };

        {
            let mut buf = *buf.borrow_mut();
            let mut udp =
                MutableUdpPacket::new(&mut buf[offset..(offset + length)]).context("udp")?;
            if udp.get_destination() != 123 {
                continue;
            }
            //udp.set_source(0);
            println!("{:?}", udp);

            println!("{:X?}", udp.to_immutable().packet());
            assert_eq!(udp.get_length(), udp.to_immutable().packet().len() as u16);

            let checksum = pnet::packet::udp::ipv4_checksum_adv(
                &udp.to_immutable(),
                &[],
                &ipv4_source,
                &ipv4_dest,
            );
            udp.set_checksum(checksum);

            println!("calculated checksum: {:X?}", checksum.to_be_bytes());
            println!("calculated checksum: {:X?}", (!checksum).to_be_bytes());
        }

        println!("writing");
        out.write_all(&buf.borrow()[0..(offset + length)])?;
    }
    Ok(())
}
