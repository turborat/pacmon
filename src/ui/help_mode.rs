use chrono::Local;
use ncurses::{ACS_HLINE, ACS_LLCORNER, ACS_LRCORNER, ACS_ULCORNER, ACS_URCORNER, ACS_VLINE, clear, COLS, LINES, mvaddch, mvhline, mvprintw, mvvline, refresh};
use crate::etc::fmt_millis;
use crate::pacstream::PacStream;
use crate::ui::{speed, UI};

pub(crate) fn print(ui: &UI, pac_vec: &Vec<PacStream>, q_depth: u64, dropped: u64, interval: u64) {
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

    let mut tt = vec![
        format!("     q depth: {:<10} drop'd: {}", q_depth, dropped),
        format!("     resolve: {:<10} pause: {:?}", ui.resolve.to_string(), ui.paused),
        format!("        recv: {:<10} sent:{:<10} interval: {:?}",
                speed(bytes_recv_last, interval), speed(bytes_sent_last, interval), interval),
        format!("   last_draw: {}", fmt_millis(ui.last_draw)),
        format!("      widths: {:?}", ui.widths),
    ];

    let cmd_strs = ui.command_info.iter()
                .map(|(c, txt)| format!("'{}':{}", c, txt))
                .collect::<Vec<String>>();

    for i in 0..cmd_strs.len()/3+1 {
        tt.push(format!("    commands: {:24} {:24} {:24}",
                        cmd_strs[i*3],
                        cmd_strs.get(i*3+1).unwrap_or(&"".to_string()),
                        cmd_strs.get(i*3+2).unwrap_or(&"".to_string())
        ));
    }

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

