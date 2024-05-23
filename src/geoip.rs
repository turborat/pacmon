use std::collections::BTreeMap;
use crate::etc::log;
use std::time::Instant;

pub struct GeoIp {
    companies: BTreeMap<String, String>,
    countries: BTreeMap<String, Location>,
}

struct Location {
    city: String,
    country: String
}

impl GeoIp {
    pub fn new() -> Self {
        let mut companies: BTreeMap<String,String> = BTreeMap::new();

        let load = Instant::now();        
        let ccc = load_companies();
        log(format!("load companies took {:?}", load.elapsed())); 

        let insert = Instant::now();        
        for cc in ccc {
          companies.insert(cc[0].to_string(), cc[1].to_string());
        }
        log(format!("insert companies took {:?}", insert.elapsed())); 

        GeoIp { companies, countries: BTreeMap::new() }
    }

    pub fn company(&self, addr:String) -> String {
        match self.companies.get(&addr) {
            Some(company) => company.to_string(),
            None => "".to_string()
        }
    }
}

#[cfg(test)]
mod tests {

    use crate::geoip::*;

    #[test]
    fn test1() {
      let start = Instant::now();
      log("hi!".to_string());
      let geoip = GeoIp::new();
      log(format!("geoip::test took {:?}", start.elapsed()));
    }

}
