use std::{panic, sync, thread};
use std::backtrace::Backtrace;
use std::cmp::{max, min, Ordering};
use std::collections::{BTreeMap, HashMap};
use std::ops::{DerefMut};
use std::sync::atomic::{AtomicI64};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use sync::atomic::Ordering::Relaxed;

use chrono::{Utc};
use ncurses::*;
use once_cell::sync::Lazy;

use crate::etc::{log, mag_fmt, millitime, str};
use crate::pacdat::StreamKey;
use crate::pacstream::PacStream;
use crate::ui::Justify::{LHS, RHS};

#[derive(Debug)]
struct UIOpt {
    q_depth: u64,
    dropped: u64,
    pac_vec: Vec<PacStream>,
    widths: Vec<i16>,
    resolve: bool,
    help: bool,
    pause: bool,
    interval: Duration,
    prev_draw: Option<Instant>
}

static OPTS: Mutex<UIOpt> = Mutex::new(UIOpt {
    q_depth: 0,
    dropped: 0,
    pac_vec: vec![],
    widths: vec![],
    resolve: true,
    help: false,
    pause: false,
    interval: Duration::from_nanos(1),
    prev_draw: None
});

static CMDS:Mutex<Lazy<HashMap<char,fn(&mut UIOpt)>>> = Mutex::new(Lazy::new(||HashMap::new()));
static HELP:Mutex<Lazy<HashMap<char,String>>> = Mutex::new(Lazy::new(||HashMap::new()));
static LAST:AtomicI64 = AtomicI64::new(0);

//put this somewhere else//
pub fn exit(code:i32, msg:String) {
    endwin();
    eprintln!("{}", msg);
    std::process::exit(code);
}

pub fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let msg = format!("DIED: {:?} {}", panic_info, Backtrace::capture());
        if msg.contains("Operation not permitted") {
            exit(-1, "Operation not permitted".to_string());
        }
        if msg.contains("Terminal too narrow") {
            exit(-2, "Terminal too narrow".to_string());
        }
        exit(-3, msg);
    }));
}

pub fn start() {
    initscr();
    curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
    refresh();

    register_cmd('q', "quit",    |_opt| exit(0, "bye".to_string()));
    register_cmd('h', "help",    |opt| opt.help = !opt.help );
    register_cmd('r', "resolve", |opt| opt.resolve = !opt.resolve );
    register_cmd(' ', "pause",   |opt| opt.pause = !opt.pause );
    register_cmd('t', "trim",    |opt| opt.widths = vec![] );

    let _ = thread::Builder::new()
        .name("pacmon:key-stroker".to_string())
        .spawn(|| keystroke_handler());
}

fn register_cmd(c:char, desc: &str, cmd:fn(&mut UIOpt)) {
    CMDS.lock().unwrap().insert(c, cmd);
    HELP.lock().unwrap().insert(c, desc.to_string());
}

pub fn should_redraw() -> bool {
    if OPTS.lock().unwrap().pause {
        false
    }
    else {
        millitime() - LAST.fetch_sub(0, Relaxed) > 4000
    }
}

#[allow(dead_code)]
fn sort_by_bytes(a:&PacStream, b:&PacStream) -> Ordering {
    let mut ret = b.bytes().cmp(&a.bytes());
    if ret.is_eq() {
        ret = b.ts_last.cmp(&a.ts_last);
    }
    ret
}

#[allow(dead_code)]
fn sort_by_last_ts(a:&PacStream, b:&PacStream) -> Ordering {
    let mut ret = b.ts_last.timestamp().cmp(&a.ts_last.timestamp());
    if ret.is_eq() {
        ret = b.bytes_last().cmp(&a.bytes_last());
    }
    if ret.is_eq() {
        ret = b.bytes().cmp(&a.bytes());
    }
    ret
}

pub fn draw(streams:&mut BTreeMap<StreamKey, PacStream>, q_depth:u64, dropped:u64) {
    if OPTS.lock().unwrap().pause {
        return
    }

    let mut pac_vec: Vec<PacStream> = streams.values().cloned().collect();
    pac_vec.sort_by(sort_by_last_ts);

    for stream in streams.values_mut() {
        stream.reset_stats();
    }

    {
        let mut opts = OPTS.lock().unwrap();
        opts.pac_vec = pac_vec;
        opts.q_depth = q_depth;
        opts.dropped = dropped;
        opts.interval = match opts.prev_draw {
            None => Duration::from_secs(1),
            Some(prev) => prev.elapsed()
        };
        opts.prev_draw = Some(Instant::now());
    }

    redraw();
}

fn redraw() {
    if COLS() != 0 && COLS() < 95 {
        log(format!("ERROR: COLS = {}", COLS()));
        panic!("Terminal too narrow!")
    }

    let (
        pac_vec,
        widths,
        q_depth,
        dropped,
        resolve,
        help,
        last_draw,
        interval,
        pause
    ) = {
        let opts = OPTS.lock().unwrap();
        (opts.pac_vec.clone(),
         opts.widths.clone(),
         opts.q_depth,
         opts.dropped,
         opts.resolve,
         opts.help,
         opts.prev_draw,
         opts.interval,
         opts.pause)
    };

    if help {
        render_help(pac_vec, widths, q_depth, dropped, resolve,
                    last_draw, pause, interval);
    }
    else {
        render_normal(pac_vec, widths, q_depth, dropped, resolve,
                      interval);
    }
    LAST.store(millitime(), Relaxed);
}

fn render_help(pac_vec: Vec<PacStream>, widths: Vec<i16>, q_depth: u64, dropped: u64,
               resolve: bool, last_draw: Option<Instant>, pause: bool, interval: Duration) {
    clear();

    mvaddch(0, 0, ACS_ULCORNER());
    mvhline(0, 1, ACS_HLINE(), COLS() - 2);
    mvaddch(0, COLS() - 1, ACS_URCORNER());
    mvvline(1, 0, ACS_VLINE(), LINES() - 2);
    mvvline(1, COLS() - 1, ACS_VLINE(), LINES() - 2);
    mvaddch(LINES() - 1, 0, ACS_LLCORNER());
    mvhline(LINES() - 1, 1, ACS_HLINE(), COLS() - 2);
    mvaddch(LINES() - 1, COLS() - 1, ACS_LRCORNER());

    let bytes_sent_last: u64 = pac_vec.iter().map(|s| s.bytes_sent_last).sum();
    let bytes_recv_last: u64 = pac_vec.iter().map(|s| s.bytes_recv_last).sum();

    let tt = vec![
        format!("     q depth: {:<8} pacs drop'd: {}", q_depth, dropped),
        format!("     resolve: {:<8} pause: {:?}", resolve.to_string(), pause),
        format!("   last_draw: {}", match last_draw {
            Some(ts) => format!("{:?}", ts),
            None => "?".to_string(),
        }),
        format!("        recv: {:<8} sent:{:<8} interval: {:?}",
                speed(bytes_recv_last, interval), speed(bytes_sent_last, interval), interval),
        format!("      widths: {:?}", widths),
        format!("    commands: {}", HELP.lock().unwrap().iter()
            .map(|(c,txt)| format!("'{}':{}", c, txt))
            .collect::<Vec<String>>()
            .join("  ")
        )
    ];

    let width = tt.iter().max_by_key(|s| s.len()).unwrap().len();
    let x_offset = (COLS() - width as i32) / 2;
    let y_offset = (LINES() - tt.len() as i32) / 2;

    for i in 0..tt.len() {
        mvprintw(i as i32 + y_offset, x_offset, &tt[i]);
    }

    mvprintw(LINES()-1, COLS()-19, &format!("{:?}", Utc::now().time()));

    refresh();
}

fn render_normal(pac_vec: Vec<PacStream>, widths: Vec<i16>, q_depth: u64, dropped: u64, resolve: bool, interval: Duration) {
    let nrows = min(pac_vec.len(), (LINES() - 2) as usize);
    let mut matrix: Vec<Vec<Cell>> = Vec::new();

    let bytes_sent_last: u64 = pac_vec.iter().map(|s| s.bytes_sent_last).sum();
    let bytes_recv_last: u64 = pac_vec.iter().map(|s| s.bytes_recv_last).sum();

    matrix.push(header(bytes_sent_last, bytes_recv_last, interval));

    for i in 0..nrows {
        let row = render(&pac_vec[i], bytes_sent_last, bytes_recv_last, resolve, interval);
        matrix.push(row);
    }

    let mut widths = compute_widths(&matrix, &widths);

    // hack to resize //
    let render_len = widths.iter().sum::<i16>();
    let deficit = COLS() as i16 - render_len;
    let comp_a = deficit / 2;
    let comp_b = deficit - comp_a;
    widths[4 /*local-host*/] += comp_a;
    widths[8 /*remote-host*/] += comp_b;

    clear();

    for i in 0..matrix.len() {
        let row = matrix.get(i).unwrap();
        let mut x = 0i32;
        let y = i;

        for j in 0..row.len() {
            let cell = row.get(j).unwrap();
            let width = widths.get(j).unwrap();

            let txt = if actual_width(&cell.txt) > *width {
                let ret = trim(*width as usize, &cell.txt);
                ret
            } else {
                cell.txt.to_string()
            };

            let offset = match cell.justify {
                LHS => 0,
                RHS => width - actual_width(&txt)
            };

            if i == 0 {
                attron(A_BOLD());
            }
            else {
                attroff(A_BOLD());
            }

            mvprintw(y as i32, x + offset as i32, &txt);

            x += *width as i32;
        }
    }

    attron(A_REVERSE());

    let footer = footer(q_depth, dropped);
    mvprintw(LINES()-1, 0, &footer);

    pad(COLS() - footer.len() as i32);

    mvprintw(LINES()-1, COLS()-8, &format!("{:?}", Utc::now().time()));

    attroff(A_REVERSE());

    refresh();

    {
        let mut opts = OPTS.lock().unwrap();
        opts.widths = widths;
    }
}

fn pad(n:i32) {
    for _ in 0..max(0, n) {
        addch(' ' as chtype);
    }
}

fn footer(q_depth:u64, dropped:u64) -> String {
    format!("q:{} dropped:{}", q_depth, dropped)
}

fn keystroke_handler() {
    loop {
        let c = getch();
        match CMDS.lock().unwrap().get(&std::char::from_u32(c as u32).unwrap()) {
            Some(cmd) => {
                {
                    let mut opts = OPTS.lock().unwrap();
                    cmd(opts.deref_mut());
                }
                redraw();
            }
            None => log(format!("getch({})", c))
        }
    }
}

enum Justify {
    LHS,
    RHS
}

struct Cell {
    txt: String,
    justify: Justify
}

impl Cell {
    fn new(justify:Justify, txt:&str) -> Self {
        Cell { txt: txt.to_string(), justify }
    }
}

fn actual_width(txt:&str) -> i16 {
    match txt.contains("%%") {
        true => (txt.len() - 1) as i16,
        false => txt.len() as i16
    }
}

fn compute_widths(matrix:&Vec<Vec<Cell>>, prev_widths:&Vec<i16>) -> Vec<i16> {
    let mut ret:Vec<i16> = Vec::new();

    for i in 0..matrix.len() {
        for j in 0..matrix.get(i).unwrap().len() {
            let cell = matrix.get(i).unwrap().get(j).unwrap();
            match ret.get(j) {
                Some(len) => {
                    ret[j] = max(actual_width(&cell.txt), *len);
                }
                None => {
                    ret.insert(j, actual_width(&cell.txt));
                }
            }
        }
    }

    for i in 0..min(prev_widths.len(), ret.len()) {
        if ret[i] < prev_widths[i] {
            ret[i] = prev_widths[i];
        }
    }

    ret
}

fn render(stream:&PacStream, total_bytes_sent: u64, total_bytes_recv: u64,
          resolve: bool, elapsed: Duration) -> Vec<Cell> {
    let mut ret:Vec<Cell> = Vec::new();
    ret.push(Cell::new(LHS, &str(stream.ip_number)));
    ret.push(Cell::new(LHS, " "));
    ret.push(Cell::new(RHS, &match stream.pid {
        Some(pid) => pid.to_string(),
        None => "?".to_string()
    }));
    ret.push(Cell::new(LHS, " "));

    if stream.foreign {
        ret.push(Cell::new(RHS, &match resolve {
            true => stream.local_host.to_string(),
            false => stream.local_addr.to_string()
        }));
    }
    else {
        ret.push(Cell::new(RHS, &match resolve {
            true => format!("<{}>", stream.proc),
            false => stream.local_addr.to_string()
        }));
    }

    ret.push(Cell::new(LHS, ":"));

    ret.push(Cell::new(LHS, &match resolve {
        true => stream.local_service.to_string(),
        false => stream.local_port.to_string()
    }));

    ret.push(Cell::new(LHS, " "));

    ret.push(Cell::new(RHS, &match resolve {
        true => trim_host(&stream.remote_host),
        false => stream.remote_addr.to_string()
    }));

    ret.push(Cell::new(LHS, ":"));

    ret.push(Cell::new(LHS, &match resolve {
        true => stream.remote_service.to_string(),
        false => stream.remote_port.to_string()
    }));

    ret.push(Cell::new(RHS, " "));
    ret.push(Cell::new(RHS, &pct_fmt(stream.bytes_recv_last as f64 / total_bytes_recv as f64)));
    ret.push(Cell::new(RHS, " "));
    ret.push(Cell::new(RHS, &speed(stream.bytes_recv_last, elapsed)));
    ret.push(Cell::new(RHS, " ("));
    ret.push(Cell::new(RHS, &mag_fmt(stream.bytes_recv)));
    ret.push(Cell::new(RHS, ") "));

    ret.push(Cell::new(RHS, &pct_fmt(stream.bytes_sent_last as f64 / total_bytes_sent as f64)));
    ret.push(Cell::new(RHS, " "));
    ret.push(Cell::new(RHS, &speed(stream.bytes_sent_last, elapsed)));
    ret.push(Cell::new(RHS, " ("));
    ret.push(Cell::new(RHS, &mag_fmt(stream.bytes_sent)));
    ret.push(Cell::new(RHS, ") "));
    ret.push(Cell::new(RHS, &stream.age()));
    ret.push(Cell::new(RHS, " "));
    ret.push(Cell::new(RHS, &stream.cc));

    if COLS() > 105 {
        ret.push(Cell::new(RHS, " "));
        ret.push(Cell::new(RHS, &stream.corp));
    }

    ret
}

fn header(total_bytes_sent: u64, total_bytes_recv: u64, elapsed: Duration) -> Vec<Cell> {
    let mut ret:Vec<Cell> = Vec::new();
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(RHS, "host|<proc>"));
    ret.push(Cell::new(LHS, ":"));
    ret.push(Cell::new(LHS, "port"));
    ret.push(Cell::new(LHS, " "));
    ret.push(Cell::new(RHS, "remote-host"));
    ret.push(Cell::new(LHS, ":"));
    ret.push(Cell::new(LHS, "port"));
    ret.push(Cell::new(RHS, ""));
    ret.push(Cell::new(RHS, "in"));
    ret.push(Cell::new(RHS, ":"));

    ret.push(Cell::new(RHS, &speed(total_bytes_recv, elapsed)));
    ret.push(Cell::new(RHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(RHS, "out"));
    ret.push(Cell::new(RHS, ":"));
    ret.push(Cell::new(RHS, &speed(total_bytes_sent, elapsed)));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(RHS, "age"));
    ret.push(Cell::new(LHS, ""));
    ret.push(Cell::new(RHS, "cc"));

    if COLS() > 105 {
        ret.push(Cell::new(RHS, " "));
        ret.push(Cell::new(RHS, "corp"));
    }

    ret
}

fn trim(len:usize, txt:&str) -> String {
    if txt.len() > len {
        let start = txt.len() - len + 1;
        let ret = "|".to_owned() + &txt[start..txt.len()];
        ret
    }
    else {
        txt.to_string()
    }
}

fn trim_host(host:&String) -> String {
    if host.len() <= 4*3+3 {
        return host.to_string();
    }

    let mut pos = host.len() - 1;
    let mut n = 0;

    while pos > 0 {
        match host.chars().nth(pos) {
            Some('.') => {
                n += 1;
                if n == 2 {
                    return host[pos+1..].to_string();
                }
            }
            _ => {}
        }
        pos -= 1;
    }

    host.to_string()
}

fn pct_fmt(pct:f64) -> String {
    if pct == 0.0 {
        "-".to_string()
    } else if pct < 0.001 {
        "~0%%".to_string()
    } else if pct < 0.01 {
        format!(".{:0.0}%%", pct * 1000.0)
    }
    else if pct == 1. {
        "***".to_string()
    }
    else {
        format!("{}%%", (pct * 100.0) as u32)
    }
}

fn speed(bytes: u64, elapsed: Duration) -> String {
    let secs = elapsed.as_millis() as f64 / 1000f64;
    if secs == 0f64 {
        mag_fmt(bytes) + "/s"
    }
    else {
        mag_fmt((bytes as f64 / secs) as u64) + "/s"
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;
    use crate::ui::{speed, Cell, compute_widths, pct_fmt, trim, trim_host};
    use crate::ui::Justify::RHS;

    #[test]
    fn test_trim() {
        assert_eq!("|ef", trim(3, "abcdef"));
        assert_eq!("abc", trim(6, "abc"));
    }

    #[test]
    fn test_compute_widths() {
        let mut matrix:Vec<Vec<Cell>> = Vec::new();
        assert_eq!(vec![] as Vec<i16>, compute_widths(&matrix, &vec![]));
        matrix.push(vec![Cell::new(RHS, "a"), Cell::new(RHS, "")]);
        assert_eq!(vec![1, 0], compute_widths(&matrix, &vec![]));
        matrix.push(vec![Cell::new(RHS, "aa"), Cell::new(RHS, "c")]);
        assert_eq!(vec![2, 1], compute_widths(&matrix, &vec![]));

        /* jagged data shouldn't be allowed really */
        matrix.push(vec![Cell::new(RHS, "aaa")]);
        assert_eq!(vec![3, 1], compute_widths(&matrix, &vec![]));
    }

    #[test]
    fn test_trim_host() {
        assert_eq!("a.b.c", trim_host(&"a.b.c".to_string()));
        assert_eq!("b.c", trim_host(&"aaaaaaaaaaaaaaaaaaa.b.c".to_string()));
        assert_eq!("b.c", trim_host(&"aaaaaaaaaaaaaaaaa.a.a.b.c".to_string()))
    }

    #[test]
    fn test_pct_fmt() {
        assert_eq!("-", pct_fmt(0.));
        assert_eq!("~0%%", pct_fmt(0.0009));
        assert_eq!(".2%%", pct_fmt(0.002));
        assert_eq!("1%%", pct_fmt(0.01));
        assert_eq!("99%%", pct_fmt(0.99));
        assert_eq!("***", pct_fmt(1.));
        assert_eq!("101%%", pct_fmt(1.01));
        assert_eq!("~0%%", pct_fmt(-0.2));
    }

    #[test]
    fn test_bps() {
        assert_eq!("123b/s", speed(123, Duration::from_millis(0)));
        assert_eq!("246b/s", speed(123, Duration::from_millis(500)));
        assert_eq!("11k/s", speed(22*1024, Duration::from_millis(2000)));
    }
}
