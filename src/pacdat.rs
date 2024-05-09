use std::fmt;
use std::fmt::Formatter;
use std::net::IpAddr;

use chrono::{DateTime, Utc};
use etherparse::IpNumber;

#[derive(PartialEq)]
pub enum Dir {
    In, Out
}

pub struct PacDat {
    pub ts: DateTime<Utc>,
    pub len: Option<u32>,
    pub ip_number: Option<IpNumber>,
    pub src_addr: Option<IpAddr>,
    pub dst_addr: Option<IpAddr>,
    pub src_port: Option<u16>,
    pub dst_port: Option<u16>,
    pub dir: Option<Dir>,
    pub foreign: Option<bool>
}

#[derive(Ord, PartialOrd, Eq, PartialEq)]
pub struct StreamKey {
    ip_number: IpNumber,
    addr1: IpAddr,
    port1: u16,
    addr2: IpAddr,
    port2: u16
}

impl PacDat {
    pub fn key(&self) -> StreamKey {
        if self.src_addr.unwrap().gt(&self.dst_addr.unwrap()) {
            StreamKey {
                ip_number: self.ip_number.unwrap(),
                addr1: self.src_addr.unwrap(),
                port1: self.src_port.unwrap(),
                addr2: self.dst_addr.unwrap(),
                port2: self.dst_port.unwrap()
            }
        } else {
            StreamKey {
                ip_number: self.ip_number.unwrap(),
                addr1: self.dst_addr.unwrap(),
                port1: self.dst_port.unwrap(),
                addr2: self.src_addr.unwrap(),
                port2: self.src_port.unwrap(),
            }
        }
    }
}

impl fmt::Display for PacDat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{} ", self.ts.format("%H:%M:%S%.6f"))?;

        match self.dir.as_ref().unwrap() {
            Dir::In => write!(f, ">> "),
            Dir::Out => write!(f, "<< ")
        }?;

        match self.ip_number {
            Some(IpNumber::TCP) => write!(f, "TCP ")?,
            Some(IpNumber::UDP) => write!(f, "UDP ")?,
            _ => panic!()
        };

        match self.dir.as_ref().unwrap() {
            Dir::In => {
                write!(f, "{}:", self.src_addr.unwrap().to_string())?;
                write!(f, "{} ", self.src_port.unwrap().to_string())?;
                if self.foreign.unwrap() {
                    write!(f, ">> {}:", self.dst_addr.unwrap().to_string())?;
                    write!(f, "{} ", self.dst_port.unwrap().to_string())?;
                }
            }
            Dir::Out => {
                write!(f, "{}:", self.dst_addr.unwrap().to_string())?;
                write!(f, "{} ", self.dst_port.unwrap().to_string())?;
                if self.foreign.unwrap() {
                    write!(f, "<< {}:", self.src_addr.unwrap().to_string())?;
                    write!(f, "{} ", self.src_port.unwrap().to_string())?;
                }
            }
        };

        write!(f, "{}", self.len.unwrap())?;

        Ok(())
    }
}
