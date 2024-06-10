mod companies;
mod locations1;
mod locations2;
mod locations3;
mod locations4;

use std::collections::BTreeMap;
use std::net::IpAddr;
use crate::etc::log;
use std::time::Instant;
use crate::subnets::{addr_to_int};

pub struct IpData {
    companies: BTreeMap<u128, Company>,
    locations: BTreeMap<u128, Location>,
}

pub struct Company {
    pub bit_mask: u128,
    pub name: String
}

pub struct Location {
    pub bit_mask: u128,
    pub city: String,
    pub country: String
}

impl IpData {
    pub fn new() -> Self {
        let mut companies: BTreeMap<u128, Company> = BTreeMap::new();
        {
            let load = Instant::now();
            let ccc = companies::load();
            log(format!("ipdata::load::companies took {:?}", load.elapsed()));

            let insert = Instant::now();
            for cc in ccc {
                companies.insert(cc.0, Company { 
                  bit_mask: bit_mask(cc.1, mask_width(cc.0)), 
                  name: cc.2.to_string() 
                });
            }
            log(format!("ipdata::insert::companies took {:?}", insert.elapsed()));
        }

        let loc_start = Instant::now();
        let mut locations: BTreeMap<u128, Location> = BTreeMap::new();
        for ccc in [locations1::load, locations2::load, locations3::load, locations4::load]
        {
            for cc in ccc() {
                locations.insert(cc.0, Location {
                    bit_mask: bit_mask(cc.1, mask_width(cc.0)),
                    country:cc.2.to_string(),
                    city:cc.3.to_string()
                });
            }
        }
        log(format!("ipdata::insert::locations took {:?}", loc_start.elapsed()));

        IpData { companies, locations }
    }

    pub fn company(&self, addr:&IpAddr) -> String {
        let t1 = Instant::now();
        let ip_int = addr_to_int(addr);
        if let Some((&subnet, &ref company)) = self.companies.range(..=ip_int).next_back() {
            log(format!("ipdata::lookup::company[{}] took {:?}", addr, t1.elapsed()));
            if same_subnet(ip_int, subnet, company.bit_mask) {
              company.name.to_string()
            }
            else {
                // compat mode - change this when confident
                "~".to_owned() + &company.name.to_string()
            }
        }
        else {
          "o".to_string()
        }
    }

    pub fn location(&self, addr:&IpAddr) -> Option<&Location> {
        let t1 = Instant::now();
        let ip_int = addr_to_int(addr);
        if let Some((&subnet, &ref location)) = self.locations.range(..=ip_int).next_back() {
            log(format!("ipdata::lookup::location[{}] took {:?}", addr, t1.elapsed()));
            if same_subnet(ip_int, subnet, location.bit_mask) {
              return Some(location);
            }
        }
        None
    }
}


fn bit_mask(bits:u32, width:u32) -> u128 {
  if bits > width {
    panic!("this should never happen");
  }
  ((1u128 << bits) - 1).wrapping_shl(width-bits)
}

fn mask_width(addr_int:u128) -> u32 {
  match addr_int <= 0xFFFFFFFF {
    true => 32,
    false => 128
  }
}

fn same_subnet(addr_int:u128, subnet_int:u128, mask:u128) -> bool {
  addr_int & mask == subnet_int & mask
}

#[cfg(test)]
mod tests {

    use crate::ipdata::*;
    use crate::subnets::addr;

    #[test]
    fn test_company() {
        let ipdata = IpData::new();
        assert_eq!("GOOGLE", ipdata.company(&addr("8.8.8.8")));
        assert_eq!("GOOGLE", ipdata.company(&addr("8.8.8.4")));
        assert_eq!("GOOGLE", ipdata.company(&addr("8.8.8.0")));
        assert_eq!("CLOUDFLARENET", ipdata.company(&addr("1.0.0.0")));

        //1.0.128.0/19
        assert_eq!("o", ipdata.company(&addr("0.1.0.0")));
        assert_eq!("TOT Public Company Limited", ipdata.company(&addr("1.0.128.3")));

        //(3758095872 /*223.255.254.0/24*/, 24, "MARINA BAY SANDS PTE LTD"),
        assert_eq!("~MARINA BAY SANDS PTE LTD", ipdata.company(&addr("224.0.0.251")));
        assert_eq!("~MARINA BAY SANDS PTE LTD", ipdata.company(&addr("239.255.255.250")));
        assert_eq!("MARINA BAY SANDS PTE LTD", ipdata.company(&addr("223.255.254.255")));
        assert_eq!("~MARINA BAY SANDS PTE LTD", ipdata.company(&addr("223.255.255.0")));
        assert_eq!("~MARINA BAY SANDS PTE LTD", ipdata.company(&addr("223.255.255.0")));
    }

    #[test]
    fn test_location() {
        let ipdata = IpData::new();
        assert_eq!("US", ipdata.location(&addr("8.8.8.8")).unwrap().country);
        assert_eq!("Suitland", ipdata.location(&addr("8.8.11.8")).unwrap().city);
    }

    #[test]
    fn test_bit_mask() {
      assert_eq!("0", format!("{:b}", bit_mask(0, 32)));
      assert_eq!("10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", format!("{:b}", bit_mask(1, 128)));
      assert_eq!("11111111111111111111111111111111000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000", format!("{:b}", bit_mask(32, 128)));
      assert_eq!("1100", format!("{:b}", bit_mask(2, 4)));
      assert_eq!("11000000", format!("{:b}", bit_mask(2, 8)));
      assert_eq!("11110000", format!("{:b}", bit_mask(4, 8)));
    }

    #[test]
    fn test_mask_width() {
        assert_eq!(32, mask_width(addr_to_int(&addr("8.8.8.8"))));      
        assert_eq!(128, mask_width(addr_to_int(&addr("2001:200:800::"))));      
    }
}
