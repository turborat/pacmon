mod corps;
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
    corps: BTreeMap<u128, Corp>,
    locations: BTreeMap<u128, Location>,
}

pub struct Corp {
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
        let mut corps: BTreeMap<u128, Corp> = BTreeMap::new();
        {
            let start = Instant::now();
            let ccc = corps::load();
            for cc in ccc {
                corps.insert(cc.0, Corp {
                  bit_mask: bit_mask(cc.1, mask_width(cc.0)), 
                  name: cc.2.to_string() 
                });
            }
            log(format!("ipdata::insert::corps took {:?}", start.elapsed()));
        }

        let mut locations: BTreeMap<u128, Location> = BTreeMap::new();
        {
          let start = Instant::now();
          for loader in [locations1::load, locations2::load, locations3::load, locations4::load]
          {
              for cc in loader() {
                  locations.insert(cc.0, Location {
                      bit_mask: bit_mask(cc.1, mask_width(cc.0)),
                      country:cc.2.to_string(),
                      city:cc.3.to_string()
                  });
              }
          }
          log(format!("ipdata::insert::locations took {:?}", start.elapsed()));
        }

        IpData { corps, locations }
    }

    pub fn company(&self, addr:&IpAddr) -> Option<String> {
        let ip_int = addr_to_int(addr);
        if let Some((&subnet, &ref corp)) = self.corps.range(..=ip_int).next_back() {
            if same_subnet(ip_int, subnet, corp.bit_mask) {
              return Some(corp.name.to_string());
            } 
        }
        None
    }

    pub fn cc(&self, addr:&IpAddr) -> String {
        let ip_int = addr_to_int(addr);
        if let Some((&subnet, &ref location)) = self.locations.range(..=ip_int).next_back() {
            if same_subnet(ip_int, subnet, location.bit_mask) {
              return location.country.to_string();
            }
        }
        return "?".to_string(); 
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
        assert_eq!("GOOGLE", ipdata.company(&addr("8.8.8.8")).unwrap());
        assert_eq!("GOOGLE", ipdata.company(&addr("8.8.8.4")).unwrap());
        assert_eq!("GOOGLE", ipdata.company(&addr("8.8.8.0")).unwrap());
        assert_eq!("CLOUDFLARENET", ipdata.company(&addr("1.0.0.0")).unwrap());

        //1.0.128.0/19
        assert_eq!(None, ipdata.company(&addr("0.1.0.0")));
        assert_eq!("TOT Public Company Limited", ipdata.company(&addr("1.0.128.3")).unwrap());

        //(3758095872 /*223.255.254.0/24*/, 24, "MARINA BAY SANDS PTE LTD"),
        assert_eq!("MARINA BAY SANDS PTE LTD", ipdata.company(&addr("223.255.254.255")).unwrap());
        assert_eq!(None, ipdata.company(&addr("224.0.0.251")));
        assert_eq!(None, ipdata.company(&addr("239.255.255.250")));
        assert_eq!(None, ipdata.company(&addr("223.255.255.0")));
    }

    #[test]
    fn test_location() {
        let ipdata = IpData::new();
        assert_eq!("US", ipdata.cc(&addr("8.8.8.8")));
        assert_eq!("US", ipdata.cc(&addr("8.8.11.8")));
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
