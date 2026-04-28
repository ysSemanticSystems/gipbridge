use anyhow::{bail, Result};
use std::time::Duration;

pub const GIP_INIT_PACKET: [u8; 5] = [0x05, 0x20, 0x00, 0x01, 0x00];

// GIP layout detection:
// We scan for where the buttons/triggers/axes block actually starts inside the 0x20 packet payload.
// This must be robust because getting the offset wrong causes "everything triggers everything".
pub const LAYOUT_DETECT_WINDOW: Duration = Duration::from_secs(2);
pub const LAYOUT_MIN_SAMPLES: u32 = 120;

#[derive(Clone, Copy, Debug, Default)]
pub struct ParsedInput {
    pub buttons: u16,
    pub lt10: u16,
    pub rt10: u16,
    pub lx: i16,
    pub ly: i16,
    pub rx: i16,
    pub ry: i16,
}

fn le_u16(b: &[u8]) -> u16 {
    u16::from_le_bytes([b[0], b[1]])
}

fn le_i16(b: &[u8]) -> i16 {
    i16::from_le_bytes([b[0], b[1]])
}

pub fn parse_gip_input_packet(pkt: &[u8], payload_offset: usize) -> Result<ParsedInput> {
    if pkt.len() < 2 {
        bail!("packet too short");
    }
    if pkt[0] != 0x20 {
        bail!("unexpected command byte 0x{:02X}", pkt[0]);
    }

    let payload = &pkt[2..];
    if payload.len() < payload_offset + 14 {
        bail!(
            "payload too short: {} bytes (need at least {})",
            payload.len(),
            payload_offset + 14
        );
    }

    let p = &payload[payload_offset..];
    Ok(ParsedInput {
        buttons: le_u16(&p[0..2]),
        lt10: le_u16(&p[2..4]) & 0x03FF,
        rt10: le_u16(&p[4..6]) & 0x03FF,
        lx: le_i16(&p[6..8]),
        ly: le_i16(&p[8..10]),
        rx: le_i16(&p[10..12]),
        ry: le_i16(&p[12..14]),
    })
}

#[derive(Clone, Debug)]
pub struct LayoutStats {
    pub samples: u32,
    trig_hi_zero: u32,
    trig_activity: u32,
    axes_non_extreme: u32,
    axes_activity: u32,
    buttons_change_count: u32,
    buttons_popcount_sum: u32,
    last_buttons: Option<u16>,
}

impl LayoutStats {
    pub fn new() -> Self {
        Self {
            samples: 0,
            trig_hi_zero: 0,
            trig_activity: 0,
            axes_non_extreme: 0,
            axes_activity: 0,
            buttons_change_count: 0,
            buttons_popcount_sum: 0,
            last_buttons: None,
        }
    }

    pub fn observe(&mut self, payload: &[u8], off: usize) {
        if payload.len() < off + 14 {
            return;
        }
        let p = &payload[off..];
        let buttons = le_u16(&p[0..2]);
        let lt_raw = le_u16(&p[2..4]);
        let rt_raw = le_u16(&p[4..6]);
        let lx = le_i16(&p[6..8]);
        let ly = le_i16(&p[8..10]);
        let rx = le_i16(&p[10..12]);
        let ry = le_i16(&p[12..14]);

        self.samples += 1;

        let lt_hi = lt_raw & !0x03FF;
        let rt_hi = rt_raw & !0x03FF;
        if lt_hi == 0 && rt_hi == 0 {
            self.trig_hi_zero += 1;
        }
        if (lt_raw & 0x03FF) > 4 || (rt_raw & 0x03FF) > 4 {
            self.trig_activity += 1;
        }

        let axes = [lx, ly, rx, ry];
        if axes.iter().all(|&v| v != i16::MIN && v != i16::MAX) {
            self.axes_non_extreme += 1;
        }
        if axes.iter().any(|&v| v.abs() > 512) {
            self.axes_activity += 1;
        }

        if let Some(prev) = self.last_buttons {
            if prev != buttons {
                self.buttons_change_count += 1;
            }
        }
        self.last_buttons = Some(buttons);
        self.buttons_popcount_sum += buttons.count_ones() as u32;
    }

    pub fn score(&self) -> i32 {
        if self.samples == 0 {
            return i32::MIN / 2;
        }
        let s = self.samples as f32;
        let trig_hi_zero = self.trig_hi_zero as f32 / s;
        let trig_activity = self.trig_activity as f32 / s;
        let axes_non_extreme = self.axes_non_extreme as f32 / s;
        let axes_activity = self.axes_activity as f32 / s;
        let btn_changes = self.buttons_change_count as f32 / s;
        let btn_pop_avg = self.buttons_popcount_sum as f32 / s;

        // Layout selection is deliberately heuristic. We want to find a stable window that behaves like:
        // - a sparse button bitfield (low popcount, occasional edges)
        // - 10-bit triggers in 16-bit words (high bits usually zero)
        // - 4× i16 axes (not saturated, some activity when sticks move)
        let mut score = 0.0f32;
        score += 6.0 * trig_hi_zero;
        score += 2.0 * trig_activity;
        score += 3.0 * axes_non_extreme;
        score += 3.0 * axes_activity;
        score += if btn_changes < 0.25 { 2.0 } else { -4.0 };
        score += if btn_pop_avg < 4.0 { 2.0 } else { -3.0 };

        (score * 100.0) as i32
    }
}

