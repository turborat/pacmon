use std::env;

mod pact;
mod etc;
mod ui;
mod subnets;
mod resolver;
mod pacdat;
mod pacstream;
mod pcap;

fn main() {
    let args: Vec<String> = env::args().collect();
    pact::run(args);
}

