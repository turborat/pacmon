use std::collections::HashSet;
use std::{env, io};
use regex::Regex;

mod pacmon;
mod etc;
mod ui;
mod subnets;
mod resolver;
mod pacdat;
mod pacstream;
mod pcap;
mod ipdata;
mod ipdata_companies;
mod ipdata_locations;

fn main() {
    let args: HashSet<String> = env::args().collect();
    if args.contains("-x") {
        eprintln!("special mode");
        special_processing()
    }
    else {
        pacmon::run();
    }
}

fn special_processing() {
    let regex = Regex::new("(^[^,]+)(,.*)").unwrap();
    for line in io::stdin().lines() {
        let txt = line.unwrap();
        if let Some(captures) = regex.captures(&txt) {
            let part1 = captures.get(1).unwrap().as_str();
            let part2 = captures.get(2).unwrap().as_str();

            match subnets::parse_subnet_to_int(part1) {
                Ok(subnet) => {
                    println!("{} /*{}*/ {}", subnet, part1, part2);
                }
                Err(msg) => {
                    // currently discard ip's that don't have a subnet
                    // better way to handle??
                    eprintln!("{}", msg)
                }
            }
        } else {
            println!("{}", &txt);
        }
    }
}
