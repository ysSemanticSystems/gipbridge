use anyhow::{anyhow, Context, Result};
use clap::Parser;
use rusb::Context as RusbContext;
use std::io::{self, Write};
use std::time::{Duration, Instant};

mod cli;
mod controllers;
mod dolphin_pipe;
mod gip;
mod mapping;
mod usb;

use crate::cli::{parse_override_vid_pid, Cli};
use crate::controllers::print_list;
use crate::gip::{
    parse_gip_input_packet, LayoutStats, GIP_INIT_PACKET, LAYOUT_DETECT_WINDOW, LAYOUT_MIN_SAMPLES,
};
use crate::mapping::{
    parsed_to_dolphin, BridgeConfig, DolphinState, StickCalibration, CALIBRATION_MAX_RADIUS,
    CALIBRATION_WINDOW_MS,
};
use crate::usb::{find_interrupt_endpoints, open_controller};

const ANALOG_EPS: f32 = 0.0025;
const ANALOG_MAX_HZ: f32 = 120.0;

fn write_line(file: &mut std::fs::File, s: &str) -> Result<()> {
    file.write_all(s.as_bytes()).context("write_all")?;
    file.write_all(b"\n").context("write_all(newline)")?;
    Ok(())
}

fn emit_button_delta(file: &mut std::fs::File, name: &str, prev: bool, now: bool) -> Result<()> {
    if prev == now {
        return Ok(());
    }
    if now {
        write_line(file, &format!("PRESS {name}"))
    } else {
        write_line(file, &format!("RELEASE {name}"))
    }
}

fn emit_state_delta(
    file: &mut std::fs::File,
    prev: DolphinState,
    now: DolphinState,
    last_analog_emit: &mut Instant,
) -> Result<()> {
    emit_button_delta(file, "A", prev.a, now.a)?;
    emit_button_delta(file, "B", prev.b, now.b)?;
    emit_button_delta(file, "X", prev.x, now.x)?;
    emit_button_delta(file, "Y", prev.y, now.y)?;
    emit_button_delta(file, "Z", prev.z, now.z)?;
    emit_button_delta(file, "START", prev.start, now.start)?;

    emit_button_delta(file, "D_UP", prev.d_up, now.d_up)?;
    emit_button_delta(file, "D_DOWN", prev.d_down, now.d_down)?;
    emit_button_delta(file, "D_LEFT", prev.d_left, now.d_left)?;
    emit_button_delta(file, "D_RIGHT", prev.d_right, now.d_right)?;

    let analog_due = last_analog_emit.elapsed() >= Duration::from_secs_f32(1.0 / ANALOG_MAX_HZ);
    let analog_changed = (prev.main_x - now.main_x).abs() > ANALOG_EPS
        || (prev.main_y - now.main_y).abs() > ANALOG_EPS
        || (prev.c_x - now.c_x).abs() > ANALOG_EPS
        || (prev.c_y - now.c_y).abs() > ANALOG_EPS
        || (prev.l - now.l).abs() > ANALOG_EPS
        || (prev.r - now.r).abs() > ANALOG_EPS;

    if analog_changed && analog_due {
        write_line(file, &format!("SET MAIN {:.4} {:.4}", now.main_x, now.main_y))?;
        write_line(file, &format!("SET C {:.4} {:.4}", now.c_x, now.c_y))?;
        write_line(file, &format!("SET L {:.4}", now.l))?;
        write_line(file, &format!("SET R {:.4}", now.r))?;
        *last_analog_emit = Instant::now();
    }

    Ok(())
}

fn i16_to_f1_for_cal(v: i16) -> f32 {
    if v == i16::MIN {
        -1.0
    } else {
        (v as f32) / 32767.0
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.list {
        print_list();
        return Ok(());
    }

    let override_vid_pid = parse_override_vid_pid(&cli)?;
    let cfg = BridgeConfig {
        y_invert: !cli.no_y_invert,
        deadzone: cli.deadzone.clamp(0.0, 0.5),
    };

    println!("gipbridge starting…");
    println!("Note: on macOS this usually needs to run as root (sudo) to claim a vendor-class USB interface.");
    let _ = io::stdout().flush();

    let pipe_path = dolphin_pipe::pipe_path(&cli.pipe_name)?;
    dolphin_pipe::ensure_fifo(&pipe_path)?;
    println!("Dolphin Pipe path: {}", pipe_path.display());
    println!(
        "In Dolphin: Controllers → Standard Controller → Configure → Device: Pipe/0/{}",
        cli.pipe_name
    );
    println!("Waiting for Dolphin to open the pipe…");
    let _ = io::stdout().flush();
    let mut pipe = dolphin_pipe::open_writer_wait(&pipe_path)?;
    println!("Pipe connected.");
    let _ = io::stdout().flush();

    let usb = RusbContext::new().context("libusb init")?;
    let (mut handle, vid, pid, name) = open_controller(&usb, override_vid_pid)?;
    println!("Opened {name} (VID=0x{vid:04X}, PID=0x{pid:04X})");

    let (iface, in_ep, out_ep) = find_interrupt_endpoints(&mut handle)?;
    println!("Claimed interface {iface}, interrupt IN=0x{in_ep:02X}, OUT=0x{out_ep:02X}");

    println!("Sending GIP init packet…");
    handle
        .write_interrupt(out_ep, &GIP_INIT_PACKET, Duration::from_millis(250))
        .context("write_interrupt(init)")?;

    println!("Reading input and writing Dolphin pipe commands…");
    let _ = io::stdout().flush();

    let mut buf = [0u8; 64];
    let mut payload_offset: Option<usize> = None;
    let mut layout_stats: Vec<LayoutStats> = Vec::new();
    let layout_start = Instant::now();

    let mut prev_state = DolphinState::default();
    let mut last_analog_emit = Instant::now();

    let mut cal = StickCalibration::default();
    let cal_start = Instant::now();

    let mut emitting = false;

    loop {
        let n = match handle.read_interrupt(in_ep, &mut buf, Duration::from_secs(1)) {
            Ok(n) => n,
            Err(rusb::Error::Timeout) => continue,
            Err(e) => return Err(anyhow!(e)).context("read_interrupt"),
        };

        let pkt = &buf[..n];

        if cli.dump {
            println!("RAW {}", hex::encode(pkt));
        }

        if pkt.first().copied() != Some(0x20) {
            continue;
        }

        if payload_offset.is_none() {
            let payload = &pkt[2..];
            let max_off = payload.len().saturating_sub(14);
            if layout_stats.len() <= max_off {
                layout_stats.resize_with(max_off + 1, LayoutStats::new);
            }
            for off in (0..=max_off).step_by(2) {
                layout_stats[off].observe(payload, off);
            }

            let samples = layout_stats.iter().map(|s| s.samples).max().unwrap_or(0);
            if layout_start.elapsed() >= LAYOUT_DETECT_WINDOW && samples >= LAYOUT_MIN_SAMPLES {
                if let Some((best_off, _)) = layout_stats
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| i % 2 == 0)
                    .map(|(i, s)| (i, s.score()))
                    .max_by_key(|&(_i, sc)| sc)
                {
                    payload_offset = Some(best_off);
                    println!("Locked GIP payload offset: {best_off} bytes");
                    println!("(Now emitting Dolphin inputs)");
                    let _ = io::stdout().flush();
                    emitting = true;
                }
            }
        }

        let off = payload_offset.unwrap_or(0);
        match parse_gip_input_packet(pkt, off) {
            Ok(parsed) => {
                if cal_start.elapsed() <= Duration::from_millis(CALIBRATION_WINDOW_MS) {
                    let lx = i16_to_f1_for_cal(parsed.lx);
                    let ly = i16_to_f1_for_cal(parsed.ly);
                    let rx = i16_to_f1_for_cal(parsed.rx);
                    let ry = i16_to_f1_for_cal(parsed.ry);

                    let lrad = (lx * lx + ly * ly).sqrt();
                    let rrad = (rx * rx + ry * ry).sqrt();
                    if lrad <= CALIBRATION_MAX_RADIUS && rrad <= CALIBRATION_MAX_RADIUS {
                        cal.n = cal.n.saturating_add(1);
                        let n = cal.n as f32;
                        cal.lx0 += (lx - cal.lx0) / n;
                        cal.ly0 += (ly - cal.ly0) / n;
                        cal.rx0 += (rx - cal.rx0) / n;
                        cal.ry0 += (ry - cal.ry0) / n;
                    }
                }

                if !emitting {
                    continue;
                }

                let now_state = parsed_to_dolphin(parsed, cal, cfg);
                if let Err(e) = emit_state_delta(&mut pipe, prev_state, now_state, &mut last_analog_emit) {
                    if let Some(ioe) = e.downcast_ref::<io::Error>() {
                        if ioe.raw_os_error() == Some(libc::EPIPE) {
                            println!("Pipe closed by reader; waiting for Dolphin to reconnect…");
                            pipe = dolphin_pipe::open_writer_wait(&pipe_path)?;
                            println!("Pipe reconnected.");
                        } else {
                            return Err(e);
                        }
                    } else {
                        return Err(e);
                    }
                }
                prev_state = now_state;
            }
            Err(e) => eprintln!("Parse error: {e:#}"),
        }
    }
}

