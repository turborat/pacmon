mod corp_mode;
mod normal_mode;
mod help_mode;
mod stats;

use std::{panic, sync};
use std::backtrace::Backtrace;
use std::cmp::{max, min, Ordering};
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicI64};
use sync::atomic::Ordering::Relaxed;

use chrono::{Local};
use ncurses::*;
use pacmon::Streams;

use crate::etc::{log, mag_fmt, millitime};
use crate::pacmon;
use crate::pacstream::PacStream;
use crate::ui::Justify::{LHS, RHS};

static REDRAW_PERIOD:AtomicI64 = AtomicI64::new(2000);
static SORT_BY:AtomicI64 = AtomicI64::new(0);

pub struct UI {
    start_time: i64,
    last_draw: i64,
    last_cols: i32,
    widths:Vec<i16>,
    commands: HashMap<char,fn(&mut UI)>,
    command_info: BTreeMap<char,String>,
    redraw_requested:bool,
    paused:bool,
    resolve:bool,
    help:bool,
    corp_mode:bool
}

impl UI {
    pub fn init() -> Self {
        set_panic_hook();
        UI {
            start_time: millitime(),
            last_draw: 0,
            last_cols: 0,
            widths: vec![],
            commands: HashMap::new(),
            command_info: BTreeMap::new(),
            redraw_requested: false,
            paused: false,
            resolve: true,
            help: false,
            corp_mode: false,
        }
    }

    pub fn show(&mut self) {
        initscr();
        curs_set(CURSOR_VISIBILITY::CURSOR_INVISIBLE);
        refresh();

        self.register_cmd('q', "quit",    |_ui| shutdown(0, "bye".to_string()));
        self.register_cmd('h', "help",    |ui| ui.help = ! ui.help);
        self.register_cmd('r', "resolve", |ui| ui.resolve = ! ui.resolve);
        self.register_cmd(' ', "pause",   |ui| ui.paused = ! ui.paused);
        self.register_cmd('t', "trim",    |ui| ui.widths.clear() );
        self.register_cmd('s', "sort",    |_ui| { let _ = SORT_BY.fetch_update(Relaxed, Relaxed, |v| Some(if v == 0 { 1 } else { 0 })); });
        self.register_cmd('c', "corps",   |ui| {
            ui.corp_mode = ! ui.corp_mode;
            ui.widths.clear();
        });
        self.register_cmd('1', "1s",      |_ui| REDRAW_PERIOD.store(1000, Relaxed));
        self.register_cmd('2', "2s",      |_ui| REDRAW_PERIOD.store(2000, Relaxed));
        self.register_cmd('3', "3s",      |_ui| REDRAW_PERIOD.store(3000, Relaxed));
        self.register_cmd('4', "4s",      |_ui| REDRAW_PERIOD.store(4000, Relaxed));
        self.register_cmd('5', "5s",      |_ui| REDRAW_PERIOD.store(5000, Relaxed));
        self.register_cmd('6', "6s",      |_ui| REDRAW_PERIOD.store(6000, Relaxed));
        self.register_cmd('7', "7s",      |_ui| REDRAW_PERIOD.store(7000, Relaxed));
        self.register_cmd('8', "8s",      |_ui| REDRAW_PERIOD.store(8000, Relaxed));
        self.register_cmd('9', "9s",      |_ui| REDRAW_PERIOD.store(9000, Relaxed));
        self.register_cmd('0', "<1s",     |_ui| REDRAW_PERIOD.store(200, Relaxed));
        self.register_cmd(66 as char, "interval--", |_ui| { REDRAW_PERIOD.fetch_add(-9, Relaxed) ;});
        self.register_cmd(65 as char, "interval++", |_ui| { REDRAW_PERIOD.fetch_add( 9, Relaxed) ;});

        self.start_time = millitime();
    }

    pub fn should_redraw(&mut self) -> bool {
        if self.redraw_requested {
            self.redraw_requested = false;
            return true;
        }

        if self.last_cols != COLS() { // unreliable ??
            log("resize detected".to_string());
            self.last_cols = COLS();
            self.widths.clear();
            return true;
        }

        if self.paused {
            return false
        }

        let now = millitime();
        let redraw_period = REDRAW_PERIOD.load(Relaxed);

        if now - self.start_time < 5000 {
            return now - self.last_draw > 99;
        }

        if now - self.last_draw > redraw_period {
            return true;
        }

        return false;
    }

    pub fn draw(&mut self, streams: &mut Streams, q_depth: u64, dropped: u64) {
        let now = millitime();
        let interval = (now - self.last_draw) as u64;

        if self.help {
            let pac_vec = to_stream_vec(&mut streams.by_stream);
            help_mode::print(self, &pac_vec, q_depth, dropped, interval);
        } else {
            if self.corp_mode {
                let pac_vec = to_stream_vec(&mut streams.by_corp);
                corp_mode::print(self, &pac_vec, q_depth, dropped, interval);
            }
            else {
                let pac_vec = to_stream_vec(&mut streams.by_stream);
                normal_mode::print(self, &pac_vec, q_depth, dropped, interval);
            }
        }

        self.last_draw = now;
    }

    fn register_cmd(&mut self, c: char, desc: &str, cmd: fn(&mut UI)) {
        match self.commands.insert(c, cmd) {
            None => {}
            Some(_) => panic!("dupe:{}", c)
        }
        self.command_info.insert(c, desc.to_string());
    }

    pub fn check_key(&mut self) {
        nodelay(stdscr(), true);
        let c = getch();
        if c != ERR {
            match self.commands.get(&std::char::from_u32(c as u32).unwrap()) {
                Some(cmd) => cmd(self),
                None => log(format!("getch({})", c))
            }
            self.request_redraw();
        }
    }

    fn store_widths(&mut self, widths: &Vec<i16>) {
        self.widths.clear();
        self.widths.extend(widths);
    }

    fn request_redraw(&mut self) {
        self.redraw_requested = true;
    }
}

fn to_stream_vec<K>(streams: &mut BTreeMap<K, PacStream>) -> Vec<PacStream> {
    let mut pac_vec: Vec<PacStream> = streams.values().cloned().collect();

    if SORT_BY.load(Relaxed) == 0 {
        pac_vec.sort_by(sort_by_last_ts);
    } else {
        pac_vec.sort_by(sort_by_bytes);
    }

    for stream in streams.values_mut() {
        stream.reset_stats();
    }

    pac_vec
}

fn print_footer(q_depth: u64, dropped: u64, paused: bool) {
    let footer = render_footer(q_depth, dropped, paused);
    attron(A_REVERSE());
    mvprintw(LINES() - 1, 0, &footer);
    pad(COLS() - footer.len() as i32);
    mvprintw(LINES() - 1, COLS() - 12, &format!("{:?}", Local::now().time()));
    attroff(A_REVERSE());
}

fn print_matrix(matrix: &mut Vec<Vec<Cell>>, widths: &mut Vec<i16>) {
    for i in 0..matrix.len() {
        let row = matrix.get(i).unwrap();
        let mut x = 0i32;
        let y = i;

        for j in 0..row.len() {
            let cell = row.get(j).unwrap();
            let width = widths.get(j).unwrap();

            let mut offset = match cell.justify {
                LHS => 0i32,
                RHS => (width - cell.width()) as i32
            };

            // if we overshoot on the LHS we truncate (left) //
            let txt = if x + offset < 0 {
                offset = -x;
                let range = (cell.txt.len() - *width as usize + 1)..cell.txt.len();
                "#".to_string() + &cell.txt[range]
            } else {
                cell.txt.to_string()
            };

            if i == 0 {
                attron(A_BOLD());
            } else {
                attroff(A_BOLD());
            }

            mvprintw(y as i32, x + offset, &txt);

            if cell.width() > *width {
                mvprintw(y as i32, x + offset - 1, " ");
            }

            x += *width as i32;
        }
    }
}

fn render_footer(q_depth: u64, dropped: u64, paused: bool) -> String {
    let interval = REDRAW_PERIOD.load(Relaxed);
    let sort = match SORT_BY.load(Relaxed) {
        0 => "time",
        1 => "total",
        _ => panic!("dead")
    };
    format!("{}x{} q:{} drop'd:{} interval:{}ms sort:{} pause:{}",
            LINES(), COLS(), q_depth, dropped, interval, sort, paused)
}


pub fn shutdown(code:i32, msg:String) {
    endwin();
    eprintln!("{}", msg);
    std::process::exit(code);
}

fn set_panic_hook() {
    panic::set_hook(Box::new(|panic_info| {
        let msg = format!("DIED: {:?} {}", panic_info, Backtrace::capture());
        if msg.contains("Operation not permitted") {
            shutdown(-1, "Operation not permitted".to_string());
        }
        if msg.contains("Terminal too narrow") {
            shutdown(-2, "Terminal too narrow".to_string());
        }
        shutdown(-3, msg);
    }));
}

fn sort_by_bytes(a:&PacStream, b:&PacStream) -> Ordering {
    let mut ret = b.bytes().cmp(&a.bytes());
    if ret.is_eq() {
        ret = b.ts_last.cmp(&a.ts_last);
    }
    ret
}

fn sort_by_last_ts(a:&PacStream, b:&PacStream) -> Ordering {
    let mut ret = b.bytes_last().cmp(&a.bytes_last());
    if ret.is_eq() {
        ret = b.ts_last.cmp(&a.ts_last);
    }
    if ret.is_eq() {
        ret = b.bytes().cmp(&a.bytes());
    }
    ret
}

fn pad(n:i32) {
    for _ in 0..max(0, n) {
        addch(' ' as chtype);
    }
}

fn compute_widths(matrix:&Vec<Vec<Cell>>, prev_widths:&Vec<i16>) -> Vec<i16> {
    let mut ret:Vec<i16> = Vec::new();
    for i in 0..matrix.len() {
        for j in 0..matrix.get(i).unwrap().len() {
            let cell = matrix.get(i).unwrap().get(j).unwrap();
            match ret.get(j) {
                Some(len) => ret[j] = max(cell.width(), *len),
                None => ret.insert(j, cell.width())
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

// returns the domain part of a url
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

fn massage_corp(txt:&mut String, target_width:usize) {
    if txt.len() > target_width {
        txt.truncate(target_width);
    }

    while txt.ends_with([' ', ',', '-']) {
        txt.truncate(txt.len()-1);
    }
}

fn pct_fmt(pct:f64) -> String {
    if pct == 0.0 || pct.is_nan() {
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

fn speed(bytes: u64, elapsed: u64) -> String {
    let secs = elapsed as f64 / 1000f64;
    if secs == 0f64 {
        mag_fmt(bytes) + "/s"
    }
    else {
        mag_fmt((bytes as f64 / secs) as u64) + "/s"
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

    fn width(&self) -> i16 {
        Cell::actual_width(&self.txt)
    }

    fn actual_width(txt:&str) -> i16 {
        match txt.contains("%%") {
            true => (txt.len() - 1) as i16,
            false => txt.len() as i16
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::{Cell, compute_widths, pct_fmt, speed, trim_host};
    use crate::ui::Justify::RHS;

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
        assert_eq!("-", pct_fmt(0f64/0f64));
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
        assert_eq!("123b/s", speed(123, 0));
        assert_eq!("246b/s", speed(123, 500));
        assert_eq!("11k/s", speed(22*1024, 2000));
    }
}
