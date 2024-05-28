use IpAddr::{V4, V6};
use std::collections::BTreeSet;
use std::net::IpAddr;
use std::sync::{Arc, mpsc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use chrono::DateTime;
use etherparse::InternetSlice::Ipv6;
use etherparse::NetSlice::Ipv4;
use etherparse::SlicedPacket;
use etherparse::TransportSlice::{Tcp, Udp};
use pcap::{Capture, Device, Packet};

use crate::etc::log;
use crate::pacdat::{Dir, PacDat};
use crate::subnets::same_subnet;

pub struct Pcap {
    q_depth: Arc<AtomicU64>,
    packets_dropped: Arc<AtomicU64>,
    tx: Sender<PacDat>,
    rx: Receiver<PacDat>
}

impl Pcap {
    pub fn new() -> Self {
        let (tx, rx) : (Sender<PacDat>, Receiver<PacDat>) = mpsc::channel();
        Pcap{
            q_depth: Arc::new(AtomicU64::new(0)),
            packets_dropped: Arc::new(AtomicU64::new(0)),
            tx,
            rx
        }
    }

    pub fn start(&self, dev:Device, interfaces: BTreeSet<(IpAddr, IpAddr)>) {
        let dropped_ref = self.packets_dropped.clone();
        let q_depth_ref = self.q_depth.clone();
        let tx_ref = self.tx.clone();
        let _ = thread::Builder::new()
            .name("pacmon:pcap".to_string())
            .spawn(move || Pcap::start_pcap(tx_ref, dev, interfaces, q_depth_ref, dropped_ref));
    }

    fn start_pcap(tx:Sender<PacDat>, dev:Device, interfaces: BTreeSet<(IpAddr, IpAddr)>, q_depth:Arc<AtomicU64>, dropped:Arc<AtomicU64>) {
        // note that we have immediate_mode=true in addition to non-zero buffer.
        // this seems to not konk out when we are eg making a fast transfer.
        let mut cap = Capture::from_device(dev).unwrap()
            .promisc(true).immediate_mode(true).buffer_size(1000*1000*1000).open().unwrap();

        loop {
            match cap.next_packet() {
                Ok(packet) => {
                    match Pcap::parse(packet, &interfaces) {
                        Some(pac_dat) => {
                            match tx.send(pac_dat) {
                                Ok(_) => q_depth.fetch_add(1, Ordering::Relaxed),
                                Err(err) => panic!("tx failed: {}", err)
                            };
                        }
                        None => {}
                    }
                }
                Err(err) => log(format!("Pcap error: {} q:{:?}", err, q_depth))
            }

            match cap.stats() {
                Ok(stats) => {
                    let all_dropped = stats.dropped + stats.if_dropped;
                    if all_dropped > 0 {
                        dropped.fetch_max(all_dropped as u64, Ordering::Relaxed);
                        log(format!("dropped {} packets", all_dropped));
                    }
                }
                Err(err) => panic!("{}", err)
            }
        }
    }

    pub fn packets_dropped(&self) -> u64 {
        self.packets_dropped.fetch_add(0, Ordering::Relaxed)
    }

    pub fn decrement_and_get_q_depth(&self) -> u64 {
        self.q_depth.fetch_sub(1, Ordering::Relaxed)
    }

    pub fn rx(&self) -> &Receiver<PacDat> {
        &self.rx
    }

    fn parse(packet: Packet, interfaces: &BTreeSet<(IpAddr, IpAddr)>) -> Option<PacDat> {
        let ts = packet.header.ts;
        let dt = DateTime::from_timestamp(ts.tv_sec, (ts.tv_usec * 1000) as u32).unwrap();

        let mut pac_dat = PacDat {
            ts: dt, len: None, ip_number: None,
            src_addr: None, dst_addr: None,
            src_port: None, dst_port: None,
            dir: None, foreign: None
        };

        match SlicedPacket::from_ethernet(&packet) {
            Ok(eth_frame) => {
                match eth_frame.net {
                    Some(Ipv4(ip_slice)) => {
                        pac_dat.src_addr = Some(V4(ip_slice.header().source_addr()));
                        pac_dat.dst_addr = Some(V4(ip_slice.header().destination_addr()));
                        pac_dat.ip_number = Some(ip_slice.payload().ip_number);
                    }
                    Some(Ipv6(ip_slice)) => {
                        pac_dat.src_addr = Some(V6(ip_slice.header().source_addr()));
                        pac_dat.dst_addr = Some(V6(ip_slice.header().destination_addr()));
                        pac_dat.ip_number = Some(ip_slice.payload().ip_number);
                    }
                    None => return None
                };

                match eth_frame.transport {
                    Some(Tcp(tcp_slice)) => {
                        pac_dat.src_port = Some(tcp_slice.source_port());
                        pac_dat.dst_port = Some(tcp_slice.destination_port());
                        pac_dat.len = Some(tcp_slice.payload().len() as u32);
                    }
                    Some(Udp(udp_slice)) => {
                        pac_dat.src_port = Some(udp_slice.source_port());
                        pac_dat.dst_port = Some(udp_slice.destination_port());
                        pac_dat.len = Some(udp_slice.payload().len() as u32)
                    }
                    _ => return None
                }
            }
            Err(err) => panic!("{}", err)
        }

        match Pcap::get_dir_foreign(&pac_dat.src_addr.unwrap(), &pac_dat.dst_addr.unwrap(), interfaces) {
            Some((dir, foreign)) => {
                pac_dat.dir = dir;
                pac_dat.foreign = foreign;
                Some(pac_dat)
            }
            None => {
                log("warn: failed to determine dir/foreign".to_string());
                None
            }
        }
    }

    fn get_dir_foreign(src_addr:&IpAddr, dst_addr:&IpAddr, interfaces:&BTreeSet<(IpAddr,IpAddr)>)
                       -> Option<(Option<Dir>, Option<bool>)> {
        for interface in interfaces {
            let (if_addr, mask) = interface;

            if if_addr == src_addr {
                return Some((Some(Dir::Out), Some(false)));
            }

            if if_addr == dst_addr {
                return Some((Some(Dir::In), Some(false)));
            }

            if same_subnet(if_addr, src_addr, mask) {
                return Some((Some(Dir::Out), Some(true)));
            }

            if same_subnet(if_addr, dst_addr, mask) {
                return Some((Some(Dir::In), Some(true)));
            }
        }

        log(format!("??:{:?} >> {:?} :: {:?}", src_addr, dst_addr, interfaces));

        None
    }
}