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
    companies: BTreeMap<u128,String>,
    locations: BTreeMap<u128,Location>,
}

pub struct Location {
    pub city: String,
    pub country: String
}

impl IpData {
    pub fn new() -> Self {
        let mut companies: BTreeMap<u128,String> = BTreeMap::new();
        {
            let load = Instant::now();
            let ccc = companies::load();
            log(format!("ipdata::load::companies took {:?}", load.elapsed()));

            let insert = Instant::now();
            for cc in ccc {
                companies.insert(cc.0, cc.1.to_string());
            }
            log(format!("ipdata::insert::companies took {:?}", insert.elapsed()));
        }

        let loc_start = Instant::now();
        let mut locations: BTreeMap<u128, Location> = BTreeMap::new();
        for ccc in [locations1::load(), locations2::load(), locations3::load(), locations4::load()]
        {
            for cc in ccc {
                locations.insert(cc.0, Location {
                    //mask: cc.1,
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
        if let Some((&_, &ref v)) = self.companies.range(..=ip_int).next_back() {
            log(format!("ipdata::lookup::company[{}] took {:?}", addr, t1.elapsed()));
            v.to_string()
        }
        else {
            panic!("Failed to determine company for {}/{}", addr, ip_int);
        }
    }

    pub fn location(&self, addr:&IpAddr) -> &Location {
        let t1 = Instant::now();
        let ip_int = addr_to_int(addr);
        if let Some((&_, &ref v)) = self.locations.range(..=ip_int).next_back() {
            log(format!("ipdata::lookup::location[{}] took {:?}", addr, t1.elapsed()));
            v
        }
        else {
            panic!("Failed to determine location for {}/{}", addr, ip_int);
        }
    }
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
        assert_eq!("TOT Public Company Limited", ipdata.company(&addr("1.1.0.0")));
    }

    #[test]
    fn test_location() {
        let ipdata = IpData::new();
        assert_eq!("US", ipdata.location(&addr("8.8.8.8")).country);
        assert_eq!("Suitland", ipdata.location(&addr("8.8.11.8")).city);
    }

}
