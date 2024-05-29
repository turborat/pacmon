use std::cmp::max;
use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::io::Write;
use std::time::{Duration, Instant};

use pcap::Device;

use etc::init_logging;

use crate::etc;
use crate::etc::log;
use crate::ui;
use crate::pacdat::{PacDat, StreamKey};
use crate::pacstream::PacStream;
use crate::pcap::Pcap;
use crate::resolver::Resolver;

pub fn run() {
    init_logging();

    let mut interfaces = BTreeSet::new();
    let dev = Device::lookup().unwrap().unwrap();
    for addr in &dev.addresses {
        if addr.addr.is_ipv4() { // no ipv6?
            log(format!("snooping {:?} / {:?}", addr.addr, addr.netmask.unwrap()));
            interfaces.insert((addr.addr, addr.netmask.unwrap()));
        }
    }

    print!("Initializing...");
    io::stdout().flush().unwrap();
    let mut resolver = Resolver::new();
    println!("done.");

    let mut streams: BTreeMap<StreamKey, PacStream> = BTreeMap::new();
    let mut packets = 0u64;
    let mut q_max = 0u64;
    let mut running = false;

    ui::set_panic_hook();

    let pcap = Pcap::new();
    pcap.start(dev, interfaces);

    loop {
        match pcap.rx().recv_timeout(Duration::from_millis(100)) {
            Ok(pac_dat) => {
                // only start curses once we get a packet
                if !running {
                    ui::start();
                    running = true;
                }

                tally(&pac_dat, &mut streams, &mut resolver);
                packets += 1 ;
                q_max = max(q_max, pcap.decrement_and_get_q_depth());

                if ui::should_redraw() {
                    let start = Instant::now();
                    ui::draw(&mut streams, q_max, pcap.packets_dropped());
                    log(format!("redraw[{}:{}] took {:?}", q_max, packets, start.elapsed()));
                    packets = 0;
                    q_max = 0;
                }
            }
            #[allow(non_snake_case)]
            Err(_recvTimeoutNonError) => {
                // duplicate code / seemed simplest
                if ui::should_redraw() {
                    let start = Instant::now();
                    ui::draw(&mut streams, q_max, pcap.packets_dropped());
                    log(format!("redraW[{}:{}] took {:?}", q_max, packets, start.elapsed()));
                    packets = 0;
                    q_max = 0;
                }
            }
        }
    }

    // t.join().unwrap();
}

fn stream_for<'a>(pac_dat:&'a PacDat, streams:&'a mut BTreeMap<StreamKey, PacStream>)
    -> &'a mut PacStream {
    let key = pac_dat.key();
    streams.entry(key).or_insert_with(|| PacStream::new(&pac_dat))
}

fn tally(pac_dat:&PacDat, streams:&mut BTreeMap<StreamKey, PacStream>, resolver:&mut Resolver) {
    let stream = stream_for(&pac_dat, streams);
    if stream.bytes() == 0 {
        stream.resolve(resolver);
    }
    stream.tally(&pac_dat);
}

