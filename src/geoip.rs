use std::collections::BTreeMap;
use std::net::IpAddr;
use crate::etc::log;
use std::time::Instant;
use crate::geoip_data;
use crate::subnets::{addr_to_int};

pub struct GeoIp {
    companies: BTreeMap<u128,String>,
    countries: BTreeMap<u128,Location>,
}

struct Location {
    city: String,
    country: String
}

impl GeoIp {
    pub fn new() -> Self {
        let mut companies: BTreeMap<u128,String> = BTreeMap::new();

        let load = Instant::now();        
        let ccc = geoip_data::load_companies();
        log(format!("geoip::load::companies took {:?}", load.elapsed()));

        let insert = Instant::now();        
        for cc in ccc {
          companies.insert(cc.0, cc.1.to_string());
        }
        log(format!("geoip::insert::companies took {:?}", insert.elapsed()));

        GeoIp { companies, countries: BTreeMap::new() }
    }

    pub fn company(&self, addr:IpAddr) -> String {
        let t1 = Instant::now();
        let ip_int = addr_to_int(addr);
        if let Some((&k, &ref v)) = self.companies.range(..=ip_int).next_back() {
            log(format!("geoip::lookup::company[{}] took {:?}", addr, t1.elapsed()));
            v.to_string()
        }
        else {
            panic!("Failed to determine company for {}/{}", addr, ip_int);
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::geoip::*;
    use crate::subnets::addr;

    #[test]
    fn test1() {
        let geoip = GeoIp::new();
        assert_eq!("GOOGLE", geoip.company(addr("8.8.8.8")));
        assert_eq!("GOOGLE", geoip.company(addr("8.8.8.4")));
        assert_eq!("GOOGLE", geoip.company(addr("8.8.8.0")));
        assert_eq!("CLOUDFLARENET", geoip.company(addr("1.0.0.0")));
        assert_eq!("TOT Public Company Limited", geoip.company(addr("1.1.0.0")));
    }

}
