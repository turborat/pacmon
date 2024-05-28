use std::collections::BTreeMap;
use std::fs;
use std::fs::{File, read_to_string};
use std::io::{BufRead, BufReader, ErrorKind, Read};
use std::net::IpAddr;
use std::time::Instant;

use etherparse::IpNumber;
use glob::glob;
use once_cell::sync::Lazy;
use regex::Regex;
use crate::etc;

use crate::etc::log;
use crate::ipdata::IpData;

// $ cat /proc/net/tcp
//   sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode
//    0: 00000000:0016 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 17335 1 0000000000000000 100 0 0 10 0
//    1: 0100007F:0277 00000000:0000 0A 00000000:00000000 00:00000000 00000000     0        0 17369 1 0000000000000000 100 0 0 10 0
//    2: 3500007F:0035 00000000:0000 0A 00000000:00000000 00:00000000 00000000   101        0 26818 1 0000000000000000 100 0 0 10 5
//    3: 0100007F:1538 00000000:0000 0A 00000000:00000000 00:00000000 00000000   129        0 25968 1 0000000000000000 100 0 0 10 0
//    4: 6D01A8C0:0016 5B01A8C0:AD3E 01 00000000:00000000 02:0009515A 00000000     0        0 87390 4 0000000000000000 20 4 31 10 89
//    5: 6D01A8C0:0016 5B01A8C0:AD42 01 00000000:00000000 02:00079E99 00000000     0        0 124241 2 0000000000000000 20 5 23 10 -1
// https://www.kernel.org/doc/Documentation/networking/proc_net_tcp.txt

static LINE_PAT: &str = r"^ *\d+: (\w{8,}:\w{4}) \w{8,}:\w{4} \w\w \w{8}:\w{8} \w{2}:\w{8} \d{8} +\d+ +\d+ +(\d+) +\d+ +\w+ +";
static LINE_REGEX: Lazy<Regex> = Lazy::new(||Regex::new(LINE_PAT).unwrap());
static PROC_REGEX: Lazy<Regex> = Lazy::new(||Regex::new(r".*/").unwrap());
static JUNK_REGEX: Lazy<Regex> = Lazy::new(||Regex::new(r"[:]").unwrap());
static WSPC_REGEX: Lazy<Regex> = Lazy::new(||Regex::new(r" .*").unwrap());

pub struct Resolver {
    dns_cache: BTreeMap<IpAddr, String>,
    pid_cache: BTreeMap<(IpNumber, IpAddr, u16), Option<u32>>,
    proc_cache: BTreeMap<u32, Option<String>>,
    services: BTreeMap<u16, String>,
    ipdata: IpData
}

impl Resolver {
    pub fn new() -> Self {
        let mut services:BTreeMap<u16, String> = BTreeMap::new();
        read_services(&mut services);

        Resolver {
            dns_cache: BTreeMap::new(),
            pid_cache: BTreeMap::new(),
            proc_cache: BTreeMap::new(),
            services,
            ipdata: IpData::new()
        }
    }

    pub fn resolve_pid(&mut self, sock_type: &IpNumber, addr: &IpAddr, port: u16) -> Option<u32> {
        let key = (*sock_type, *addr, port);
        *self.pid_cache.entry(key).or_insert_with(|| {
            let start = Instant::now();
            let ret = match resolve_socket_inode(sock_type, addr, port) {
                Some(inode) => pid_for_socket_inode(inode),
                None => None
            };
            log(format!("resolve_pid[{}:{}/{}] -> {:?} took {:?}", addr, port, etc::str(*sock_type), ret, start.elapsed()));
            ret
        })
    }

    pub fn resolve_proc(&mut self, pid: u32) -> Option<String> {
        self.proc_cache.entry(pid).or_insert_with(|| proc_for_pid(pid)).clone()
    }

    pub fn resolve_host(&mut self, addr: IpAddr) -> String {
        match self.dns_cache.get(&addr) {
            Some(host) => return host.to_string(),
            None => {}
        }

        let start = Instant::now();

        let host = match dns_lookup::lookup_addr(&addr) {
            Ok(host) => host,
            Err(_) => addr.to_string()
        };

        log(format!("resolve_host[{}] took {:?}", addr, start.elapsed()));

        self.dns_cache.insert(addr, host.to_string());

        host
    }

    pub fn resolve_service(&self, port: u16) -> String {
        match self.services.get(&port) {
            Some(service) => service.to_string(),
            None => port.to_string()
        }
    }
}

fn read_services(services:&mut BTreeMap<u16, String>) {
    let fh = File::open("/etc/services").unwrap();
    let reader = BufReader::new(fh);
    let regex = Regex::new(r"^(\S+)\s+(\d+)/.*").unwrap();

    for line_res in reader.lines() {
        let line = line_res.unwrap();
        match regex.captures(&line) {
            Some(captures) => {
                let name = captures.get(1).unwrap().as_str();
                let number = captures.get(2).unwrap().as_str().parse::<u16>().unwrap();
                services.insert(number, name.to_string());
            }
            None => {}
        };
    }
}

fn proc_for_pid(pid:u32) -> Option<String> {
    let start = Instant::now();
    let path = format!("/proc/{}/cmdline", pid);
    let mut fh = match File::open(&path) {
        Ok(fh) => fh,
        Err(msg) => {
            log(format!("!!file::open {}: {}", path, msg));
            return None
        }
    };

    let mut cmd = String::new();
    fh.read_to_string(&mut cmd).unwrap();

    let parts:Vec<&str> = cmd.split(&['\0', ' ']).collect();
    let mut proc = parts.get(0).unwrap().to_string();

    proc = PROC_REGEX.replace_all(&proc, "").to_string();
    proc = JUNK_REGEX.replace_all(&proc, "").to_string();
    proc = WSPC_REGEX.replace_all(&proc, "").to_string();

    log(format!("proc_for_pid[{}] -> {} took {:?}", pid, proc, start.elapsed()));

    Some(proc)
}

fn pid_for_socket_inode(inode:u32) -> Option<u32> {
    let seeking = format!("socket:[{}]", inode);

    for e in glob("/proc/*/fd/*").expect("glob failed") {
        match e {
            Ok(path) => {
                let link = match fs::read_link(&path) {
                    Ok(buf) => buf,
                    Err(err) => {
                        match err.kind() {
                            ErrorKind::NotFound => {}
                            _ => log(format!("read_link::{}", err))
                        }
                        continue
                    }
                };

                if link.to_str().unwrap() == &seeking {
                    match path.components().nth(2).unwrap().as_os_str().to_str() {
                        Some(str) => return Some(str.parse::<u32>().unwrap()),
                        None => panic!("Failed to parse {:?}", link)
                    };
                }
            }
            Err(err) => match err.error().kind() {
                ErrorKind::PermissionDenied => {},
                _ => log(format!("glob_err::{}", err))
            }
        }
    }
    return None;
}

fn resolve_socket_inode(sock_type:&IpNumber, addr:&IpAddr, port:u16) -> Option<u32> {
    let start = Instant::now();
    let key = create_key(addr, port);

    let file = "/proc/net/".to_string() + match *sock_type {
        IpNumber::TCP => if addr.is_ipv6() { "tcp6" } else { "tcp" },
        IpNumber::UDP => if addr.is_ipv6() { "udp6" } else { "udp" },
        _ => panic!("ffs")
    };

    let mut header = true;
    for line in read_to_string(&file).unwrap().lines() {
        if header {
            header = false;
        } else {
            let (hex_ip_port, inode) = extract_hex_ip_port_inode(line);
            if hex_ip_port == key {
                log(format!("resolve_socket_inode[{}:{}] took {:?}", addr, port, start.elapsed()));
                return Some(inode)
            }
        }
    }
    return None
}

fn extract_hex_ip_port_inode(line:&str) -> (String, u32) {
    match LINE_REGEX.captures(line) {
        Some(captures) => {
            let hex_ip_port = captures.get(1).unwrap().as_str();
            match captures.get(2).unwrap().as_str().parse::<u32>() {
                Ok(inode) => return (hex_ip_port.to_string(), inode),
                Err(err) => panic!("{}", err)
            }
        }
        _ => panic!("Failed to parse [{}]", &line)
    }
}

fn create_key(ip:&IpAddr, port:u16) -> String {
    let port_str = format!("{:04x}", port);
    match ip {
        IpAddr::V4(ipv4) => format!("{}:{}", to_hex_nbo(&ipv4.octets()), port_str),
        IpAddr::V6(ipv6) => format!("{}:{}", to_hex_nbo(&ipv6.octets()), port_str)
    }.to_uppercase()
}

fn to_hex_nbo(octets:&[u8]) -> String {
    let mut ret = String::new();
    for i in 0..octets.len() {
        let ri = octets.len() - 1 - i;
        ret.push_str(format!("{:02X}", octets[ri]).as_str());
    }
    ret
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, TcpListener, UdpSocket};
    use std::net::IpAddr::V4;
    use std::str::FromStr;
    use std::thread;
    use std::time::Duration;

    use etherparse::IpNumber;

    use crate::etc::log;
    use crate::resolver::{create_key, extract_hex_ip_port_inode, pid_for_socket_inode, proc_for_pid, resolve_socket_inode, Resolver, to_hex_nbo};

    fn _resolve_proc_old(sock_type: &IpNumber, addr: &IpAddr, port: u16) -> String {
        match Resolver::new().resolve_pid(sock_type, addr, port) {
            None => "?".to_string(),
            Some(pid) => match Resolver::new().resolve_proc(pid) {
                Some(proc) => proc,
                None => "?".to_string()
            }
        }
    }

    #[test]
    fn test_to_hex_nbo() {
        let oo: [u8; 4] = [192, 168, 1, 91];
        assert_eq!("5B01A8C0", to_hex_nbo(&oo));

        let oo2: [u8; 4] = Ipv4Addr::from_str("192.168.1.91").unwrap().octets();
        assert_eq!("5B01A8C0", to_hex_nbo(&oo2));

        let oov6: [u8; 16] = Ipv6Addr::from_str("fe80::2d56:de1f:eb7a").unwrap().octets();
        assert_eq!("7AEB1FDE562D000000000000000080FE", to_hex_nbo(&oov6));
    }

    #[test]
    fn test_create_key() {
        let addr = V4(Ipv4Addr::from_str("192.168.1.109").unwrap());
        let port = 22;
        assert_eq!("6D01A8C0:0016", create_key(&addr, port));
        // todo: ipv6
    }

    #[test]
    fn test_create_key_2() {
        let addr = V4(Ipv4Addr::from_str("127.0.0.1").unwrap());
        let port = 42431;
        assert_eq!("0100007F:A5BF", create_key(&addr, port));
        // todo: ipv6
    }

    #[test]
    fn test_ipv6() {
        println!("{}", format!("{:04x}", 35822));
    }

    #[test]
    fn test_extract() {
        let line = "13389: 00000000000000000000000000000000:8D7B 00000000000000000000000000000000:0000 07 00000000:00000000 00:00000000 00000000   122        0 18726 2 ffff912787654440 0";
        assert_eq!(("00000000000000000000000000000000:8D7B".to_string(), 18726), extract_hex_ip_port_inode(&line));
    }

    #[test]
    fn test_resolve_socket_inode_tcp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        println!("{}", addr);
        let inode = resolve_socket_inode(&IpNumber::TCP, &addr.ip(), addr.port());
        println!("{:?}", inode);
        assert!(inode.unwrap() > 0);
    }

    #[test]
    #[ignore]
    fn test_resolve_socket_inode_tcp_v6() {
        let listener = TcpListener::bind("[fe80::2d56:de1f:eb7a:1140]:0").unwrap();
        let addr = listener.local_addr().unwrap();
        println!("{}", addr);
        let inode = resolve_socket_inode(&IpNumber::TCP, &addr.ip(), addr.port());
        println!("{:?}", inode);
        assert!(inode.unwrap() > 0);
    }

    #[test]
    fn test_resolve_socket_inode_udp() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        println!("{}", addr);
        let inode = resolve_socket_inode(&IpNumber::UDP, &addr.ip(), addr.port());
        println!("{:?}", inode);
        assert!(inode.unwrap() > 0);
    }

    #[ignore]
    #[test]
    fn udp_tester() {
        let sock = UdpSocket::bind("192.168.1.109:0").unwrap();
        let addr = sock.local_addr().unwrap();

        log(format!("addr = {}", addr));

        loop {
            log(format!("sending udp packet to {}", addr));
            UdpSocket::send_to(&sock,"hello".as_bytes(), addr).unwrap();
            thread::sleep(Duration::from_millis(100));
        }
    }

    #[test]
    fn test_resolve_proc_tcp() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let proc = _resolve_proc_old(&IpNumber::TCP, &addr.ip(), addr.port());
        assert!(proc.starts_with("pacmon-"));
    }

    #[test]
    fn test_resolve_proc_udp() {
        let listener = UdpSocket::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        println!("{}", addr);
        let proc = _resolve_proc_old(&IpNumber::UDP, &addr.ip(), addr.port());
        assert!(proc.starts_with("pacmon-"));
    }

    #[test]
    fn test_pid_for_inode() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        println!("bound to {}", addr);

        let inode = resolve_socket_inode(&IpNumber::TCP, &addr.ip(), addr.port()).unwrap();
        println!("socket inode: {:?}", inode);
        assert!(inode > 0);

        println!("pid (of test): {}", std::process::id());

        let pid = pid_for_socket_inode(inode);
        assert_eq!(pid.unwrap(), std::process::id());
    }

    #[test]
    fn test_proc_for_pid() {
        let pid = std::process::id();
        let proc = proc_for_pid(pid);
        println!("{:?}", proc);
        assert_eq!(true, proc.unwrap().starts_with("pacmon-"));
    }

    #[test]
    fn test_resolve_host() {
        // todo: determine ip dynamically //
        let addr = V4(Ipv4Addr::from_str("192.168.1.109").unwrap());
        let host = Resolver::new().resolve_host(addr);
        assert_eq!("DEV", host);
    }

    #[test]
    fn test_resolve_service() {
        assert_eq!("http", Resolver::new().resolve_service(80));
        assert_eq!("9934", Resolver::new().resolve_service(9934));
    }
}