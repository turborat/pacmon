use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::net::IpAddr::{V4, V6};

pub fn subnet(addr:&IpAddr, mask:&IpAddr) -> Option<IpAddr> {
    match addr {
        V4(v4addr) => {
            match mask {
                V4(v4mask) => Some(V4(v4_subnet(&v4addr, &v4mask))),
                V6(_) => None
            }
        }
        V6(v6addr) => {
            match mask {
                V4(_) => None,
                V6(v6mask) => Some(V6(v6_subnet(&v6addr, &v6mask))),
            }
        }
    }
}

fn v4_subnet(addr:&Ipv4Addr, mask:&Ipv4Addr) -> Ipv4Addr {
    let mut oo:[u8;4] = Default::default();
    for i in 0..4 {
        oo[i] = addr.octets()[i] & mask.octets()[i];
    }
    Ipv4Addr::new(oo[0], oo[1], oo[2], oo[3])
}

fn v6_subnet(addr:&Ipv6Addr, mask:&Ipv6Addr) -> Ipv6Addr {
    let mut oo:[u16;8] = Default::default();
    for i in 0..8 {
        oo[i] = addr.segments()[i] & mask.segments()[i];
    }
    Ipv6Addr::new(oo[0], oo[1], oo[2], oo[3], oo[4], oo[5], oo[6], oo[7])
}

pub fn same_subnet(addr1:&IpAddr, addr2:&IpAddr, mask:&IpAddr) -> bool {
    match subnet(addr1, mask) {
        None => false,
        Some(subnet1) => match subnet1 {
            V4(v4_subnet1) => {
                match subnet(addr2, mask) {
                    Some(V4(v4_subnet2)) => v4_subnet2 == v4_subnet1,
                    _ => false
                }
            }
            V6(v6_subnet1) => {
                match subnet(addr2, mask) {
                    Some(V6(v6_subnet2)) => v6_subnet2 == v6_subnet1,
                    _ => false
                }
            }
        },
    }
}

pub fn parse_subnet(txt:&str) -> u128 {
    fn to_int(oo:&[u8]) -> u128 {
        let mut ret = 0u128;
        for o in oo {
            ret <<= 8;
            ret += *o as u128;
        }
        ret
    }

    fn to_mask(mask_bits:u8, mask_len:u8) -> u128 {
        let mut ret = 0u128;
        for _ in 0..mask_bits {
            ret <<= 1;
            ret += 1;
        }
        ret << (mask_len - mask_bits)
    }

    let parts:Vec<_> = txt.split("/").collect();
    if parts.len() != 2 {
        panic!("?!:{}", txt);
    }

    let addr_str = parts[0];
    let mask_bits = parts[1].parse::<u8>().unwrap();

    match addr_str.parse::<Ipv4Addr>() {
        Ok(addr) => {
            to_int(&addr.octets()) & to_mask(mask_bits, 32)
        },
        Err(_) => match addr_str.parse::<Ipv6Addr>() {
            Ok(addr) => {
                to_int(&addr.octets()) & to_mask(mask_bits, 128)
            },
            Err(err) => panic!("{}: {}", err, addr_str)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use IpAddr::V4;
    use crate::subnets::subnet;

    #[test]
    fn test_subnet() {
        let addr = V4(Ipv4Addr::new(12, 12, 12, 12));
        let mask = V4(Ipv4Addr::new(0xFF, 0xFF, 0, 0));
        let expected = V4(Ipv4Addr::new(12, 12, 0, 0));
        assert_eq!(Some(expected), subnet(&addr, &mask));
    }
}


