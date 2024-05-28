use std::collections::BTreeMap;
use std::net::IpAddr;
use crate::etc::log;
use std::time::Instant;
use crate::ipdata_companies;
use crate::subnets::{addr_to_int};

pub struct IpData {
    companies: BTreeMap<u128,String>,
    countries: BTreeMap<u128,Location>,
}

struct Location {
    city: String,
    country: String
}

impl IpData {
    pub fn new() -> Self {
        let mut companies: BTreeMap<u128,String> = BTreeMap::new();

        let load = Instant::now();        
        let ccc = ipdata_companies::load();
        log(format!("ipdata::load::companies took {:?}", load.elapsed()));

        let insert = Instant::now();        
        for cc in ccc {
          companies.insert(cc.0, cc.1.to_string());
        }
        log(format!("ipdata::insert::companies took {:?}", insert.elapsed()));

        IpData { companies, countries: BTreeMap::new() }
    }

    pub fn company(&self, addr:IpAddr) -> String {
        let t1 = Instant::now();
        let ip_int = addr_to_int(addr);
        if let Some((&k, &ref v)) = self.companies.range(..=ip_int).next_back() {
            log(format!("ipdata::lookup::company[{}] took {:?}", addr, t1.elapsed()));
            v.to_string()
        }
        else {
            panic!("Failed to determine company for {}/{}", addr, ip_int);
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::ipdata::*;
    use crate::subnets::addr;

    #[test]
    fn test1() {
        let ipdata = IpData::new();
        assert_eq!("GOOGLE", ipdata.company(addr("8.8.8.8")));
        assert_eq!("GOOGLE", ipdata.company(addr("8.8.8.4")));
        assert_eq!("GOOGLE", ipdata.company(addr("8.8.8.0")));
        assert_eq!("CLOUDFLARENET", ipdata.company(addr("1.0.0.0")));
        assert_eq!("TOT Public Company Limited", ipdata.company(addr("1.1.0.0")));
    }

}
