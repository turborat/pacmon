use std::net::IpAddr;
use std::time::{Duration, UNIX_EPOCH};

use chrono::{DateTime, Utc};
use etherparse::IpNumber;

use crate::etc;
use crate::pacdat::{Dir, PacDat};
use crate::resolver::Resolver;

#[derive(Clone)]
#[derive(Debug)]
pub struct PacStream {
    pub proc: String,
    pub pid: Option<u32>,
    pub bytes_sent: u64,
    pub bytes_sent_last: u64,
    pub bytes_recv: u64,
    pub bytes_recv_last: u64,
    pub local_port: u16,
    pub local_addr: IpAddr,
    pub local_host: String,
    pub local_service: String,
    pub remote_port: u16,
    pub remote_addr: IpAddr,
    pub remote_host: String,
    pub remote_service: String,
    pub cc: String,
    pub corp: String,
    pub ts_last: DateTime<Utc>,
    pub foreign: bool,              // foreign = from another local host
    pub local_traffic: bool,        // is the traffic just on our subnet
    pub ip_number: IpNumber,
    pub packets_in: u64,
    pub packets_out: u64
}

impl PacStream {
    pub fn new(pac_dat:&PacDat) -> PacStream {
        let (local_addr, local_port, remote_addr, remote_port) = if pac_dat.dir == Some(Dir::Out) {
            (pac_dat.src_addr, pac_dat.src_port, pac_dat.dst_addr, pac_dat.dst_port)
        } else {
            (pac_dat.dst_addr, pac_dat.dst_port, pac_dat.src_addr, pac_dat.src_port)
        };

        PacStream {
            proc: "tbd".to_string(),
            pid: None,
            bytes_sent: 0,
            bytes_sent_last: 0,
            bytes_recv: 0,
            bytes_recv_last: 0,
            local_port: local_port.unwrap(),
            local_addr: local_addr.unwrap(),
            local_host: "tbd".to_string(),
            local_service: "tbd".to_string(),
            remote_port: remote_port.unwrap(),
            remote_addr: remote_addr.unwrap(),
            remote_host: "tbd".to_string(),
            remote_service: "tbd".to_string(),
            cc: "?".to_string(),
            corp: "?".to_string(),
            ts_last: pac_dat.ts,
            foreign: pac_dat.foreign.unwrap(),
            local_traffic: pac_dat.local_traffic.unwrap(),
            ip_number: pac_dat.ip_number.unwrap(),
            packets_in: 0,
            packets_out:0
        }
    }

    pub fn tally(&mut self, pac_dat:&PacDat) {
        let len = pac_dat.len.unwrap() as u64;
        if pac_dat.dir == Some(Dir::Out) {
            self.bytes_sent += len;
            self.bytes_sent_last += len;
            self.packets_out += 1;
        }
        else {
            self.bytes_recv += len;
            self.bytes_recv_last += len;
            self.packets_in += 1;
        }
        self.ts_last = pac_dat.ts;
    }

    pub fn reset_stats(&mut self) {
        self.bytes_sent_last = 0;
        self.bytes_recv_last = 0;
    }

    pub fn bytes(&self) -> u64 {
        self.bytes_sent + self.bytes_recv
    }

    pub fn bytes_last(&self) -> u64 {
        self.bytes_sent_last + self.bytes_recv_last
    }

    pub fn age(&self) -> String {
        // in case the interval is > 1s
        if self.bytes_last() > 0 {
            return ".".to_string();
        }

        let unix_time = self.ts_last.timestamp();
        let instant = UNIX_EPOCH + Duration::from_secs(unix_time as u64);
        let duration = instant.elapsed().unwrap();
        etc::fmt_duration(duration)
    }

    // todo: put this in ::new //
    pub fn resolve(&mut self, resolver: &mut Resolver) -> PacStream {
        if self.foreign {
            self.proc = "this should never be displayed".to_string();
        }
        else {
            self.pid = resolver.resolve_pid(&self.ip_number, &self.local_addr, self.local_port);
            self.proc = match self.pid {
                Some(pid) => match resolver.resolve_proc(pid) {
                    Some(proc) => proc,
                    None => "-".to_string()
                },
                None => "-".to_string()
            };
        };
        self.local_host = resolver.resolve_host(self.local_addr).to_string();
        self.remote_host = resolver.resolve_host(self.remote_addr).to_string();
        self.local_service = resolver.resolve_service(self.local_port);
        self.remote_service = resolver.resolve_service(self.remote_port);
        if self.local_traffic {
            self.cc = "-".to_string();
            self.corp = "-".to_string();
        }
        else {
            self.cc = resolver.resolve_cc(&self.remote_addr);
            self.corp = match resolver.resolve_company(&self.remote_addr) {
                Some(corp) => corp,
                None => "?".to_string()
            };
        }
        self.to_owned()
    }
}
