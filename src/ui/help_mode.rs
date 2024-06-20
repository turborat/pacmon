use std::sync::atomic::Ordering::Relaxed;
use chrono::Local;
use ncurses::{ACS_HLINE, ACS_LLCORNER, ACS_LRCORNER, ACS_ULCORNER, ACS_URCORNER, ACS_VLINE, clear, COLS, LINES, mvaddch, mvhline, mvprintw, mvvline, refresh};
use crate::etc::fmt_millis;
use crate::pacstream::PacStream;
use crate::ui::{CMD_INFO, PAUSED, RESOLVE, speed};

pub(crate) fn print(pac_vec: &Vec<PacStream>, widths: &Vec<i16>, q_depth: u64, dropped: u64, interval: u64, last_draw: i64) {
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
    let resolve = RESOLVE.fetch_and(true, Relaxed);
    let pause = PAUSED.fetch_and(true, Relaxed);

    let mut tt = vec![
        format!("     q depth: {:<8} pacs drop'd: {}", q_depth, dropped),
        format!("     resolve: {:<8} pause: {:?}", resolve.to_string(), pause),
        format!("   last_draw: {}", fmt_millis(last_draw)),
        format!("        recv: {:<8} sent:{:<8} interval: {:?}",
                speed(bytes_recv_last, interval), speed(bytes_sent_last, interval), interval),
        format!("      widths: {:?}", widths),
        format!("    commands: {}", CMD_INFO.lock().unwrap().iter()
            .map(|(c, txt)| format!("'{}':{}", c, txt))
            .collect::<Vec<String>>()
            .join("  ")
        )
    ];

    for t in &mut tt {
        t.truncate(COLS() as usize - 2);
    }

    let width = tt.iter().max_by_key(|s| s.len()).unwrap().len();
    let x_offset = (COLS() - width as i32) / 2;
    let y_offset = (LINES() - tt.len() as i32) / 2;

    for i in 0..tt.len() {
        mvprintw(i as i32 + y_offset, x_offset, &tt[i]);
    }

    mvprintw(LINES() - 1, COLS() - 19, &format!("{:?}", Local::now().time()));

    refresh();
}

