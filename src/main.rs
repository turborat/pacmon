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

fn main() {
    let args: HashSet<String> = env::args().collect();
    if args.contains("-x") {
        special_processing()
    }
    else {
        pacmon::run(args);
    }
}

fn special_processing() {
    let regex = Regex::new("(^[^,]+),(.*)").unwrap();
    let mut status = 0;
    for line in io::stdin().lines() {
        let txt = line.unwrap();
        if let Some(captures) = regex.captures(&txt) {
            let addr = captures.get(1).unwrap().as_str();
            let rest = captures.get(2).unwrap().as_str();
            match subnets::parse_subnet_to_int(addr) {
                Ok(subnet) => {
                    println!("{},{},{}", addr, subnet, rest);
                }
                Err(msg) => {
                    eprintln!("{} - dropping line", msg);
                    status -= 1;
                }
            }
        } else {
            eprintln!("Unknown input [{}] - dropping line", &txt);
            status -= 1;
        }
    }
    std::process::exit(status); 
}
