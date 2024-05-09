use std::env;

mod pacmon;
mod etc;
mod ui;
mod subnets;
mod resolver;
mod pacdat;
mod pacstream;
mod pcap;

fn main() {
    let args: Vec<String> = env::args().collect();
    pacmon::run(args);
}

