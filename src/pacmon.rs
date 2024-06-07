use std::cmp::max;
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::io;
use std::io::Write;
use std::net::IpAddr;
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

pub fn run(args: HashSet<String>) {
    if args.contains("-l") {
        init_logging();
    }

    let mut interfaces = BTreeSet::new();
    let dev = Device::lookup().unwrap().unwrap();
    for addr in &dev.addresses {
        if addr.addr.is_ipv4() { // no ipv6?
            log(format!("snooping {:?} / {:?}", addr.addr, addr.netmask.unwrap()));
            interfaces.insert((addr.addr, addr.netmask.unwrap()));
        }
    }

    print!("+ipdata..");
    io::stdout().flush().unwrap();
    let mut resolver = Resolver::new();
    println!("done.\n~pcap..");

    let mut streams: BTreeMap<StreamKey, PacStream> = BTreeMap::new();
    let mut packets = 0u64;
    let mut last_packets_dropped = 0u64;
    let mut q_max = 0u64;
    let mut running = false;

    ui::set_panic_hook();

    let pcap = Pcap::new();
    pcap.start(dev);

    loop {
        match pcap.rx().recv_timeout(Duration::from_millis(100)) {
            Ok(mut pac_dat) => {
                // only start curses once we get a packet
                if !running {
                    ui::start();
                    running = true;
                }

                tally(&mut pac_dat, &mut streams, &mut resolver, &interfaces);
                packets += 1 ;
                q_max = max(q_max, pcap.decrement_and_get_q_depth());
            }
            Err(_recv_timeout_non_error) => {
            }
        }

        if ui::should_redraw() {
            let start = Instant::now();
            let dropped = pcap.packets_dropped();
            let dropped_curr = dropped - last_packets_dropped;

            ui::draw(&mut streams, q_max.clone(), dropped_curr);

            log(format!("redraw[qMax:{} packets:{}] took {:?}", q_max, packets, start.elapsed()));

            if dropped_curr > 0 {
                log(format!("err: dropped {} packets", dropped_curr));
            }

            packets = 0;
            q_max = 0;
            last_packets_dropped = dropped;
        }
    }

    // t.join().unwrap();
}

fn stream_for<'a>(pac_dat:&'a PacDat, streams:&'a mut BTreeMap<StreamKey, PacStream>)
    -> &'a mut PacStream {
    let key = pac_dat.key();
    streams.entry(key).or_insert_with(|| PacStream::new(&pac_dat))
}

fn tally(pac_dat: &mut PacDat, streams:&mut BTreeMap<StreamKey, PacStream>, resolver:&mut Resolver, interfaces:&BTreeSet<(IpAddr, IpAddr)>) {
    match Pcap::get_dir_foreign(&pac_dat.src_addr.unwrap(), &pac_dat.dst_addr.unwrap(), interfaces) {
        Some((dir, foreign, local_traffic)) => {
            pac_dat.dir = Some(dir);
            pac_dat.foreign = Some(foreign);
            pac_dat.local_traffic = Some(local_traffic);
        }
        None => {
            log("warn: failed to determine dir/foreign - ignoring packet".to_string());
            return;
        }
    }

    let stream = stream_for(&pac_dat, streams);
    if stream.bytes() == 0 {
        stream.resolve(resolver);
    }
    stream.tally(&pac_dat);
}

