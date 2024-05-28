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
mod geoip;
mod geoip_data;

fn main() {
    let args: HashSet<String> = env::args().collect();
    println!("args: {:?}", args);
    if args.contains("-x") {
        special_processing()
    }
    else {
        pacmon::run();
    }
}

fn special_processing() {
    let regex = Regex::new("(.*\\[)\"([^\"]+)\"(.*)").unwrap();
    for line in io::stdin().lines() {
        let txt = line.unwrap();
        if let Some(captures) = regex.captures(&txt) {
            let part1 = captures.get(1).unwrap().as_str();
            let part2 = captures.get(2).unwrap().as_str();
            let part3 = captures.get(3).unwrap().as_str();

            match subnets::parse_subnet_to_int(part2) {
                Ok(subnet) => {
                    println!("{}{} /*{}*/ {}", part1, subnet, part2, part3);
                }
                Err(msg) => eprintln!("{}", msg)
            }
        } else {
            println!("{}", &txt);
        }
    }
}
