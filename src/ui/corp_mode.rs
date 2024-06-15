use std::cmp::min;
use ncurses::{clear, LINES, refresh};
use crate::pacstream::PacStream;
use crate::ui;
use crate::ui::{Cell, compute_widths, UI};
use crate::ui::Justify::RHS;

pub(crate) fn print(pac_vec: &Vec<PacStream>, prev_widths: Vec<i16>, q_depth: u64, dropped: u64, interval: u64) {
    let nrows = min(pac_vec.len(), (LINES() - 2) as usize);
    let mut matrix: Vec<Vec<Cell>> = Vec::new();

    let bytes_sent_last: u64 = pac_vec.iter().map(|s| s.bytes_sent_last).sum();
    let bytes_recv_last: u64 = pac_vec.iter().map(|s| s.bytes_recv_last).sum();

    let mut header: Vec<Cell> = Vec::new();
    header.push(Cell::new(RHS, "corp"));

    matrix.push(header);

    for i in 0..nrows {
//            let row = self.render_row(&pac_vec[i], bytes_sent_last, bytes_recv_last, resolve, interval);
//            matrix.push(row);
    }

    let mut widths = compute_widths(&matrix, &vec![]);
    //widths[0] = 20;

    clear();

    ui::print_matrix(&mut matrix, &mut widths);

    ui::print_footer(q_depth, dropped);

    refresh();
}

