use crate::etc::mag_fmt;
use crate::pacstream::PacStream;
use crate::ui::{Cell, pct_fmt, speed};
use crate::ui::Justify::{LHS, RHS};

pub fn add_headers(row: &mut Vec<Cell>, total_bytes_sent: u64, total_bytes_recv: u64, interval: u64) {
    row.push(Cell::new(RHS, "in"));
    row.push(Cell::new(RHS, ":"));
    row.push(Cell::new(RHS, &speed(total_bytes_recv, interval)));
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(LHS, " "));
    row.push(Cell::new(LHS, " "));
    row.push(Cell::new(RHS, "out"));
    row.push(Cell::new(RHS, ":"));
    row.push(Cell::new(RHS, &speed(total_bytes_sent, interval)));
    row.push(Cell::new(LHS, " "));
    row.push(Cell::new(LHS, " "));
}

pub fn add(row: &mut Vec<Cell>, stream: &PacStream, total_bytes_sent: u64, total_bytes_recv: u64, interval: u64) {
    row.push(Cell::new(RHS, &pct_fmt(stream.bytes_recv_last as f64 / total_bytes_recv as f64)));
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(RHS, &speed(stream.bytes_recv_last, interval)));
    row.push(Cell::new(RHS, " ("));
    row.push(Cell::new(RHS, &format!("{})", mag_fmt(stream.bytes_recv))));
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(RHS, &pct_fmt(stream.bytes_sent_last as f64 / total_bytes_sent as f64)));
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(RHS, &speed(stream.bytes_sent_last, interval)));
    row.push(Cell::new(RHS, " ("));
    row.push(Cell::new(RHS, &format!("{})", mag_fmt(stream.bytes_sent))));
}