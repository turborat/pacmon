use std::cmp::min;
use std::sync::atomic::Ordering::Relaxed;
use ncurses::{clear, COLS, LINES, refresh};
use ui::{compute_widths, print_footer, print_matrix};
use crate::pacstream::PacStream;
use crate::ui;
use crate::ui::{Cell, massage_corp, pct_fmt, RESOLVE, speed, stats, trim_host, UI, WIDTHS};
use crate::ui::Justify::{LHS, RHS};

pub(crate) fn print(pac_vec: &Vec<PacStream>, prev_widths: Vec<i16>, q_depth: u64, dropped: u64, interval: u64) {
    let nrows = min(pac_vec.len(), (LINES() - 2) as usize);
    let mut matrix: Vec<Vec<Cell>> = Vec::new();

    let bytes_sent_last: u64 = pac_vec.iter().map(|s| s.bytes_sent_last).sum();
    let bytes_recv_last: u64 = pac_vec.iter().map(|s| s.bytes_recv_last).sum();

    let resolve = RESOLVE.fetch_and(true, Relaxed);
    matrix.push(render_header(bytes_sent_last, bytes_recv_last, interval, resolve));

    for i in 0..nrows {
        let row = render_row(&pac_vec[i], bytes_sent_last, bytes_recv_last, resolve, interval);
        matrix.push(row);
    }

    let mut widths = compute_widths(&matrix, &prev_widths);

    // hack hack hack hack hack hack hack - to line things up //
    let render_len = widths.iter().sum::<i16>();
    let deficit = COLS() as i16 - render_len;
    let local_col = 0;
    let remote_col = 4;
    let total = widths[local_col] + widths[remote_col] + deficit;
    let ratio = 0.45;   // local :: remote
    widths[local_col] = (total as f32 * ratio) as i16;
    widths[remote_col] = total - widths[local_col];

    clear();

    print_matrix(&mut matrix, &mut widths);

    print_footer(q_depth, dropped);

    refresh();

    {
        let mut prev_widths = WIDTHS.lock().unwrap();
        prev_widths.clear();
        prev_widths.extend(widths);
    }
}

fn render_row(stream: &PacStream, total_bytes_sent: u64, total_bytes_recv: u64, resolve: bool, elapsed: u64) -> Vec<Cell> {
    let mut row: Vec<Cell> = Vec::new();

    if stream.foreign {
        row.push(Cell::new(RHS, &match resolve {
            true => stream.local_host.to_string(),
            false => stream.local_addr.to_string()
        }));
    } else {
        row.push(Cell::new(RHS, &match resolve {
            true => format!("<{}>", stream.proc),
            false => stream.local_addr.to_string()
        }));
    }

    row.push(Cell::new(LHS, ":"));

    row.push(Cell::new(LHS, &match resolve {
        true => stream.local_service.to_string(),
        false => stream.local_port.to_string()
    }));

    row.push(Cell::new(LHS, " "));

    row.push(Cell::new(RHS, &match resolve {
        true => trim_host(&stream.remote_host),
        false => stream.remote_addr.to_string()
    }));

    row.push(Cell::new(LHS, ":"));

    row.push(Cell::new(LHS, &match resolve {
        true => {
            let mut ss = stream.remote_service.to_string();
            ss.truncate(6);
            ss
        },
        false => stream.remote_port.to_string()
    }));

    row.push(Cell::new(RHS, " "));
    stats::add(&mut row, &stream, total_bytes_sent, total_bytes_recv, elapsed);
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(RHS, &stream.age()));
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(RHS, &stream.cc));

    let mut corp = stream.corp.to_string();
    massage_corp(&mut corp, (COLS() as f32 * 0.14) as usize);
    row.push(Cell::new(RHS, ""));
    row.push(Cell::new(RHS, &corp));

    row
}

fn render_header(total_bytes_sent: u64, total_bytes_recv: u64, elapsed: u64, resolve: bool) -> Vec<Cell> {
    let mut row: Vec<Cell> = Vec::new();
    row.push(Cell::new(RHS, "host|<proc>"));
    row.push(Cell::new(LHS, ":"));
    row.push(Cell::new(LHS, "port"));
    row.push(Cell::new(LHS, " "));
    row.push(Cell::new(RHS, "remote-host"));
    row.push(Cell::new(LHS, ":"));
    row.push(Cell::new(LHS, match resolve {
        true => "svc",
        false => "port"
    }));
    row.push(Cell::new(RHS, " "));
    stats::add_headers(&mut row, total_bytes_sent, total_bytes_recv, elapsed);
    row.push(Cell::new(LHS, ""));
    row.push(Cell::new(RHS, "age"));
    row.push(Cell::new(LHS, ""));
    row.push(Cell::new(RHS, "cc"));
    row.push(Cell::new(RHS, " "));
    row.push(Cell::new(RHS, "corp"));
    row
}