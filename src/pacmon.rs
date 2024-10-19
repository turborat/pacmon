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
use crate::pacdat::{PacDat, StreamKey};
use crate::pacstream::PacStream;
use crate::pcap::Pcap;
use crate::resolver::Resolver;
use crate::ui::UI;

pub struct Streams {
    pub by_stream: BTreeMap<StreamKey, PacStream>,
    pub by_corp: BTreeMap<String, PacStream>
}

impl Streams {
    fn new() -> Self {
        Streams{
            by_stream: BTreeMap::new(),
            by_corp: BTreeMap::new()
        }
    }
}

pub fn run(args: HashSet<String>) {
    if args.contains("-l") {
        init_logging();
    }

    let mut interfaces = BTreeSet::new();
    let dev = Device::lookup().unwrap().unwrap();
    for addr in &dev.addresses {
        if addr.addr.is_ipv4() {
            log(format!("snooping {:?} / {:?} (IPv4 only)", addr.addr, addr.netmask.unwrap()));
            interfaces.insert((addr.addr, addr.netmask.unwrap()));
        }
    }

    print!("+ipdata..");
    io::stdout().flush().unwrap();
    let mut resolver = Resolver::new();
    println!("done.\n~pcap..");

    let mut streams = Streams::new();
    let mut packets = 0u64;
    let mut last_dropped = 0u64;
    let mut q_max = 0u64;
    let mut running = false;

    let mut ui = UI::init();

    let pcap = Pcap::new();
    pcap.start(dev);

    loop {
        match pcap.rx().recv_timeout(Duration::from_millis(10)) {
            Ok(mut pac_dat) => {
                // only start curses once we get a packet
                if !running {
                    ui.show();
                    running = true;
                }

                tally(&mut pac_dat, &mut streams, &mut resolver, &interfaces);
                packets += 1 ;
                q_max = max(q_max, pcap.decrement_and_get_q_depth());
            }
            Err(_recv_timeout_non_error) => {
            }
        }

        ui.check_key();

        if ui.should_redraw() {
            let start = Instant::now();
            let dropped = pcap.packets_dropped();
            let dropped_curr = dropped - last_dropped;

            ui.draw(&mut streams, q_max, dropped_curr);

            log(format!("redraw[q:{} packets:{}] took {:?}", q_max, packets, start.elapsed()));

            if dropped_curr > 0 {
                log(format!("err: dropped {} packets", dropped_curr));
            }

            packets = 0;
            q_max = 0;
            last_dropped = dropped;
        }
    }
}

fn stream_for<'a,K>(key:K, pac_dat:&'a PacDat, streams:&'a mut BTreeMap<K, PacStream>, resolver:&mut Resolver)
    -> &'a mut PacStream where K: Ord {
    streams.entry(key).or_insert_with(|| PacStream::new(&pac_dat).resolve(resolver))
}

fn tally(pac_dat: &mut PacDat, streams: &mut Streams, resolver:&mut Resolver, interfaces:&BTreeSet<(IpAddr, IpAddr)>) {
    // do this off the pcap thread in hopes of dropping fewer packets
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

    {   // tally by stream //
        let key = pac_dat.key();
        stream_for(key, pac_dat, &mut streams.by_stream, resolver).tally(&pac_dat);
    }

    {   // tally by corp //
        let key = match resolver.resolve_company(&pac_dat.remote_addr()) {
            Some(corp) => corp,
            None => resolver.resolve_host(pac_dat.remote_addr())
        };
        stream_for(key, pac_dat, &mut streams.by_corp, resolver).tally(&pac_dat);
    }
}

