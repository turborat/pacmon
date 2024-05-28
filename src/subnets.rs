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

pub fn addr_to_int(addr:&IpAddr) -> u128 {
    match addr {
        V4(v4addr) => octets_to_int(&v4addr.octets()),
        V6(v6addr) => octets_to_int(&v6addr.octets())
    }
}

fn octets_to_int(oo:&[u8]) -> u128 {
    let mut ret = 0u128;
    for o in oo {
        ret <<= 8;
        ret += *o as u128;
    }
    ret
}

pub fn parse_subnet_to_int(txt:&str) -> Result<u128,String> {
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
        return Err(format!("?!:{}", txt));
    }

    let addr_str = parts[0];
    let mask_bits = parts[1].parse::<u8>().unwrap();

    match addr_str.parse::<Ipv4Addr>() {
        Ok(addr) => return Ok(octets_to_int(&addr.octets()) & to_mask(mask_bits, 32)),
        Err(_) => {}
    };

    match addr_str.parse::<Ipv6Addr>() {
        Ok(addr) => return Ok(octets_to_int(&addr.octets()) & to_mask(mask_bits, 128)),
        Err(_) => {}
    };

    Err(format!("Failed to parse [{}]", addr_str))
}

pub fn addr(txt:&str) -> IpAddr {
    match txt.parse::<Ipv4Addr>() {
        Ok(addr) => V4(addr),
        Err(_) => V6(txt.parse::<Ipv6Addr>().unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::net::{IpAddr, Ipv4Addr};
    use IpAddr::V4;
    use crate::subnets::{addr, addr_to_int, parse_subnet_to_int, subnet};

    #[test]
    fn test_subnet() {
        let addr = V4(Ipv4Addr::new(12, 12, 12, 12));
        let mask = V4(Ipv4Addr::new(0xFF, 0xFF, 0, 0));
        let expected = V4(Ipv4Addr::new(12, 12, 0, 0));
        assert_eq!(Some(expected), subnet(&addr, &mask));
    }

    #[test]
    fn test_parse_subnet() {
        assert_eq!(0, parse_subnet_to_int("0.0.0.0/32").unwrap());
        assert_eq!(0x08080808, parse_subnet_to_int("8.8.8.8/32").unwrap());
        assert_eq!(0x08080000, parse_subnet_to_int("8.8.8.8/16").unwrap());
        assert_eq!(0xFF000000, parse_subnet_to_int("255.255.255.255/8").unwrap());
        assert_eq!(42540766452641154071740215577757643572, parse_subnet_to_int("2001:0db8:85a3:0000:0000:8a2e:0370:7334/128").unwrap());
        assert_eq!(42540766452641154071740063647526813696, parse_subnet_to_int("2001:0db8:85a3:0000:0000:8a2e:0370:7334/64").unwrap());
        assert_eq!(42535295865117307932921825928971026432, parse_subnet_to_int("2001:0db8:85a3:0000:0000:8a2e:0370:7334/8").unwrap());
    }

    #[test]
    fn test_addr_to_int() {
        assert_eq!(0, addr_to_int(&addr("0.0.0.0")));
        assert_eq!(0x08080808, addr_to_int(&addr("8.8.8.8")));
        assert_eq!(0x00FF0000, addr_to_int(&addr("0.255.0.0")));
        assert_eq!(42540766452641154071740215577757643572, addr_to_int(&addr("2001:0db8:85a3:0000:0000:8a2e:0370:7334")));
    }

}


