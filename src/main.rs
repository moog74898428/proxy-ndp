/*
   Copyright 2023 moog <moog74898428@outlook.jp>

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/
use pnet::datalink::Channel::Ethernet;
use pnet::datalink::NetworkInterface;
use pnet::packet::ethernet::{EtherTypes, EthernetPacket, MutableEthernetPacket};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv6::{Ipv6Packet, MutableIpv6Packet};
use pnet::packet::icmpv6;
use pnet::packet::icmpv6::{Icmpv6Packet, MutableIcmpv6Packet, Icmpv6Types, Icmpv6Code};
use pnet::packet::icmpv6::ndp::{MutableNdpOptionPacket, MutableNeighborAdvertPacket, NeighborAdvertFlags, NeighborSolicitPacket};
use pnet::packet::icmpv6::ndp::NdpOptionTypes::TargetLLAddr;
use pnet::packet::{Packet, MutablePacket, FromPacket};
use pnet::util::MacAddr;
use std::net::Ipv6Addr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use std::vec;

static USAGE: &str = "USAGE: proxy-ndp <NETWORK INTERFACE> <TARGET MAC ADDRESS> <PREFIX> <PREFIX LENGTH>";

struct Configuration {
    target_mac_address: MacAddr,
    prefix: Ipv6Addr,
    prefix_length: u8,
}

fn main() {
    let mut args = std::env::args().skip(1);
    let iface_name0 = match args.next() {
        Some(n) => n,
        None => { eprintln!("{}", USAGE); std::process::exit(1); }
    };
    let target_mac_address: MacAddr = match args.next() {
        Some(n) => n.parse().expect("Invalid MAC address"),
        None => { eprintln!("{}", USAGE); std::process::exit(1); }
    };
    let prefix: Ipv6Addr = match args.next() {
        Some(n) => n.parse().expect("Invalid IPv6 address"),
        None => { eprintln!("{}", USAGE); std::process::exit(1); }
    };
    let prefix_length: u8 = match args.next() {
        Some(n) => n.parse().expect("Invalid prefix length"),
        None => { eprintln!("{}", USAGE); std::process::exit(1); }
    };
    let config = Configuration { target_mac_address, prefix, prefix_length };
    let interface0 = Arc::new(
        pnet::datalink::interfaces()
            .into_iter()
            .find(|iface| iface.name == iface_name0)
            .unwrap(),
    );
    let mut threads = vec![];
    let (mut tx0, mut rx0) = match pnet::datalink::channel(&interface0, Default::default()) {
        Ok(Ethernet(tx, rx)) => (tx, rx),
        Ok(_) => panic!("unhandled channel type"),
        Err(e) => panic!("unable to create channel: {}", e),
    };
    let (rx_sender, rx_receiver) = std::sync::mpsc::channel::<Box<[u8]>>();
    let (tx_sender, tx_receiver) = std::sync::mpsc::channel::<Box<[u8]>>();
    let rx_receiver = Arc::new(Mutex::new(rx_receiver));
    let tx_receiver = Arc::new(Mutex::new(tx_receiver));
    threads.push(std::thread::Builder::new()
        .name(format!("receiver thread{}", 1))
        .spawn({
            move || loop {
                match rx0.next() {
                    Ok(packet_in) => {
                        rx_sender.send(packet_in.into()).unwrap();
                    }
                    Err(e) => panic!("unable to receive packet: {}", e),
                }
            }
        })
        .unwrap()
    );
    threads.push(std::thread::Builder::new()
        .name(format!("processor thread{}", 1))
        .spawn({
            let itf0 = Arc::clone(&interface0);
            move || loop {
                let receiver = rx_receiver.lock().unwrap();
                let packet_in = receiver.recv().unwrap();
                process_ethernet(1, &config, &packet_in, &itf0, &tx_sender);
            }
        })
        .unwrap()
    );
    threads.push(std::thread::Builder::new()
        .name(format!("sender thread{}", 1))
        .spawn(move || loop {
            let receiver = tx_receiver.lock().unwrap();
            let packet_out = receiver.recv().unwrap();
            tx0.send_to(&packet_out, None);
        })
        .unwrap()
    );
    for t in threads {
        t.join().unwrap();
    }
}

fn mask_ipv6_addr(ipv6_addr: &Ipv6Addr, prefix_length: u8) -> [u8; 16] {
    let mut result = [0u8; 16];
    let mut i = 0;
    while i < prefix_length / 8 {
        result[i as usize] = ipv6_addr.octets()[i as usize];
        i += 1;
    }
    if prefix_length % 8 != 0 {
        result[i as usize] = ipv6_addr.octets()[i as usize] & (0xff << (8 - prefix_length % 8));
    }
    result
}

fn process_ipv6(_id: u8, config: &Configuration, ethernet: &EthernetPacket, _itf0: &NetworkInterface, tx: &Sender<Box<[u8]>>) {
    let header = Ipv6Packet::new(ethernet.payload()).expect("bad Ethernet payload");
    match header.get_next_header() {
        IpNextHeaderProtocols::Icmpv6 => {
            let rx_icmpv6_packet = Icmpv6Packet::new(header.payload()).expect("bad ICMPv6 payload");
            if rx_icmpv6_packet.get_icmpv6_type() == Icmpv6Types::NeighborSolicit {
                let rx_ndp_packet = NeighborSolicitPacket::new(rx_icmpv6_packet.packet()).expect("bad NDP payload");
                println!("R:Ipv6:Icmpv6: {:?}", rx_ndp_packet);
                let source_mac = config.target_mac_address;
                let target_ipv6_address = rx_ndp_packet.get_target_addr();
                let prefix = config.prefix;
                let prefix_length = config.prefix_length;
                let masked_prefix = mask_ipv6_addr(&prefix, prefix_length);
                let masked_target_ipv6_address = mask_ipv6_addr(&target_ipv6_address, prefix_length);
                if masked_prefix == masked_target_ipv6_address {
                    let mut eth_buffer = vec![0u8; 14 + 40 + 32];
                    let mut eth_packet = MutableEthernetPacket::new(&mut eth_buffer).expect("Failed to create Ethernet packet");
                    eth_packet.set_ethertype(EtherTypes::Ipv6);
                    eth_packet.set_source(source_mac);
                    eth_packet.set_destination(ethernet.get_source());
                    let mut ipv6_packet = MutableIpv6Packet::new(eth_packet.payload_mut()).expect("Failed to create IPv6 packet");
                    ipv6_packet.set_version(6);
                    ipv6_packet.set_next_header(IpNextHeaderProtocols::Icmpv6);
                    ipv6_packet.set_source(target_ipv6_address);
                    ipv6_packet.set_destination(header.get_source());
                    ipv6_packet.set_hop_limit(255);
                    ipv6_packet.set_payload_length(32);
                    let mut ndp_option_buffer = [0u8; 8];
                    let mut ndp_option_packet = MutableNdpOptionPacket::new(&mut ndp_option_buffer).expect("Failed to create NDP Option packet");
                    ndp_option_packet.set_option_type(TargetLLAddr);
                    ndp_option_packet.set_length(1);
                    ndp_option_packet.set_data(&[source_mac.0, source_mac.1, source_mac.2, source_mac.3, source_mac.4, source_mac.5]);
                    let mut ndp_packet = MutableNeighborAdvertPacket::new(ipv6_packet.payload_mut()).expect("Failed to create Neighbor Advertisement packet");
                    ndp_packet.set_icmpv6_type(Icmpv6Types::NeighborAdvert);
                    ndp_packet.set_icmpv6_code(Icmpv6Code(0));
                    ndp_packet.set_flags(NeighborAdvertFlags::Solicited | NeighborAdvertFlags::Override);
                    ndp_packet.set_target_addr(target_ipv6_address);
                    ndp_packet.set_options(&[ndp_option_packet.from_packet()]);
                    let mut icmpv6_packet = MutableIcmpv6Packet::new(ipv6_packet.payload_mut()).expect("Failed to create ICMPv6 packet");
                    icmpv6_packet.set_checksum(icmpv6::checksum(&icmpv6_packet.to_immutable(), &target_ipv6_address, &header.get_source()));
                    println!("S:Ipv6:Icmpv6: {:?}", icmpv6_packet);
                    tx.send(eth_buffer.into()).unwrap();
                }
            }
        }
        _ => {
            // println!("Unknown: {:?}", header);
        }
    }
}

fn process_ethernet(id: u8, config: &Configuration, packet_in: &[u8], itf0: &NetworkInterface, tx: &Sender<Box<[u8]>>) {
    let ethernet = EthernetPacket::new(packet_in).unwrap();
    match ethernet.get_ethertype() {
        EtherTypes::Ipv6 => process_ipv6(id, config, &ethernet, itf0, tx),
        _ => {}
    }
}
