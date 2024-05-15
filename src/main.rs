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

fn main() {
    let args: HashSet<String> = env::args().collect();
    pacmon::run();
}

