use std::time::Duration;

use std::{fs};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::Mutex;
use chrono::{Local, Utc};

use etherparse::IpNumber;

pub fn mag_fmt(value: u64) -> String {
    fn scale_num(value: u64, base: u64) -> String {
        let fp = value as f64 / base as f64;
        let rounded = (fp + 0.5f64) as u64;
        if rounded < 10 {
            format!("{:.1}", fp)
        } else {
            format!("{}", rounded)
        }
    }

    if value == 0u64 {
        String::from("-")
    } else if value > 999_999_999 {
        scale_num(value, 1_000_000_000) + "g"
    } else if value > 999_999 {
        scale_num(value, 1_000_000) + "m"
    } else if value > 999 {
        scale_num(value, 1_000) + "k"
    } else {
        scale_num(value, 1) + "b"
    }
}

pub fn str(ip_number: IpNumber) -> String {
    match ip_number {
        IpNumber::UDP => "UDP".to_string(),
        IpNumber::TCP => "TCP".to_string(),
        _ => panic!("{:?}", ip_number)
    }
}

static mut LOGFILE: Mutex<Option<File>> = Mutex::new(None);

pub fn log(msg: String) {
    unsafe {
        if let Some(ref mut file) = &mut *LOGFILE.lock().unwrap() {
            let ts = Local::now().format("%H:%M:%S%.3f");
            writeln!(file, "{} {}", ts, msg).unwrap();
        }
    }
}

pub fn init_logging() {
    let fname = "pacmon.log";
    println!(">{}", fname);
    let _ = fs::remove_file(fname);
    let file = OpenOptions::new().append(true).create(true).open(fname).unwrap();
    unsafe {
        match LOGFILE.lock() {
            Ok(mut guard) => *guard = Some(file),
            Err(err) => panic!("failed to create {}: {}", fname, err)
        };
    }
}

pub fn fmt_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    if secs < 1 {
        ".".to_string()
    } else if secs < 100 {
        format!("{}s", secs)
    } else if secs <= 99 * 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h", secs / 3600)
    }
}

pub fn millitime() -> i64 {
    let now = Utc::now();
    now.timestamp() * 1000 + now.timestamp_millis()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use crate::etc::{fmt_duration, mag_fmt};

    #[test]
    fn test_mag_fmt() {
        assert_eq!("543b", mag_fmt(543));
        assert_eq!("1.0k", mag_fmt(1000));
        assert_eq!("1.2k", mag_fmt(1234));
        assert_eq!("1.3k", mag_fmt(1294));
        assert_eq!("13m", mag_fmt(12944723));
        assert_eq!("1.3m", mag_fmt(1294472));
        assert_eq!("1.0g", mag_fmt(1_000_000_000));

        assert_eq!("10b", mag_fmt(10));
        assert_eq!("10k", mag_fmt(10_000));
        assert_eq!("10m", mag_fmt(10_000_000));
        assert_eq!("10g", mag_fmt(10_000_000_000));

        assert_eq!("1.0b", mag_fmt(1));
        assert_eq!("1.0k", mag_fmt(1000));
        assert_eq!("1.0m", mag_fmt(1000_000));
        assert_eq!("1.0g", mag_fmt(1000_000_000));

        assert_eq!("10m", mag_fmt(9962084));
    }

    #[test]
    fn test_fmt_elapsed() {
        assert_eq!(".", fmt_duration(Duration::from_secs(0)));
        assert_eq!("3s", fmt_duration(Duration::from_secs(3)));
        assert_eq!("60s", fmt_duration(Duration::from_secs(60)));
        assert_eq!("99s", fmt_duration(Duration::from_secs(99)));
        assert_eq!("1m", fmt_duration(Duration::from_secs(100)));
        assert_eq!("99m", fmt_duration(Duration::from_secs(99*60)));
        assert_eq!("1h", fmt_duration(Duration::from_secs(100*60)));
        assert_eq!("5h", fmt_duration(Duration::from_secs(5*60*60)));
        assert_eq!("99h", fmt_duration(Duration::from_secs(99*60*60)));
    }
}