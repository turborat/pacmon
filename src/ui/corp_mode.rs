use std::cmp::min;
use ncurses::{clear, LINES, refresh};
use ui::{print_footer, print_matrix};
use crate::pacstream::PacStream;
use crate::ui;
use crate::ui::{Cell, compute_widths, stats, UI};
use crate::ui::Justify::{LHS, RHS};

pub(crate) fn print(pac_vec: &Vec<PacStream>, prev_widths: Vec<i16>, q_depth: u64, dropped: u64, interval: u64) {
    let nrows = min(pac_vec.len(), (LINES() - 2) as usize);
    let mut matrix: Vec<Vec<Cell>> = Vec::new();

    let bytes_sent_last: u64 = pac_vec.iter().map(|s| s.bytes_sent_last).sum();
    let bytes_recv_last: u64 = pac_vec.iter().map(|s| s.bytes_recv_last).sum();

    let mut header: Vec<Cell> = Vec::new();
    header.push(Cell::new(LHS, "corp"));
    header.push(Cell::new(RHS, " "));
    header.push(Cell::new(RHS, "cc"));
    header.push(Cell::new(RHS, " "));
    stats::add_headers(&mut header, bytes_sent_last, bytes_recv_last, interval);
    matrix.push(header);

    for i in 0..nrows {
        let mut row: Vec<Cell> = Vec::new();
        let pac = &pac_vec[i];
        if pac.corp.len() < 2 {
            row.push(Cell::new(LHS, &pac.remote_host));
        }
        else {
            row.push(Cell::new(LHS, &pac.corp));
        }
        row.push(Cell::new(LHS, " "));
        row.push(Cell::new(LHS, &pac.cc));
        row.push(Cell::new(LHS, " "));
        stats::add(&mut row, pac, bytes_sent_last, bytes_recv_last, interval);
        matrix.push(row);
    }

    let mut widths = compute_widths(&matrix, &prev_widths);

    clear();

    print_matrix(&mut matrix, &mut widths);

    print_footer(q_depth, dropped);

    refresh();

    UI::store_widths(&widths);
}

