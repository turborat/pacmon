use std::collections::HashSet;
use std::env;

mod pacmon;
mod etc;
mod ui;
mod subnets;
mod resolver;
mod pacdat;
mod pacstream;
mod pcap;
mod geoip;
mod geoip_data;

fn main() {
    let _args: HashSet<String> = env::args().collect();
    pacmon::run();
}

