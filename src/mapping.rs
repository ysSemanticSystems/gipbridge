use crate::gip::ParsedInput;

#[derive(Clone, Copy, Debug)]
pub struct BridgeConfig {
    pub y_invert: bool,
    pub deadzone: f32,
}

// Stick calibration.
pub const CALIBRATION_WINDOW_MS: u64 = 750;
pub const CALIBRATION_MAX_RADIUS: f32 = 0.25; // only learn center when stick is near center

#[derive(Clone, Copy, Debug, Default)]
pub struct DolphinState {
    pub a: bool,
    pub b: bool,
    pub x: bool,
    pub y: bool,
    pub z: bool,
    pub start: bool,
    pub d_up: bool,
    pub d_down: bool,
    pub d_left: bool,
    pub d_right: bool,
    pub main_x: f32,
    pub main_y: f32,
    pub c_x: f32,
    pub c_y: f32,
    pub l: f32,
    pub r: f32,
}

fn clamp01(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

fn clamp11(v: f32) -> f32 {
    if v.is_nan() {
        0.0
    } else if v < -1.0 {
        -1.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

fn norm_i16_to_f1(v: i16) -> f32 {
    // Map i16 to [-1, 1]. Keep symmetric behavior around 0.
    if v == i16::MIN {
        -1.0
    } else {
        (v as f32) / 32767.0
    }
}

fn norm_trig10_to_01(v10: u16) -> f32 {
    clamp01((v10.min(1023) as f32) / 1023.0)
}

#[derive(Clone, Copy, Debug, Default)]
pub struct StickCalibration {
    pub lx0: f32,
    pub ly0: f32,
    pub rx0: f32,
    pub ry0: f32,
    pub n: u32,
}

pub fn apply_radial_deadzone(x: f32, y: f32, dz: f32) -> (f32, f32) {
    let dz = dz.clamp(0.0, 0.5);
    let r = (x * x + y * y).sqrt();
    if r <= dz {
        return (0.0, 0.0);
    }
    // Rescale so output starts at 0 at the edge of the deadzone.
    let k = (r - dz) / (1.0 - dz);
    let s = if r > 0.0 { k / r } else { 0.0 };
    (clamp11(x * s), clamp11(y * s))
}

/// Map GIP `0x20` state into Dolphin's pipe protocol state.
///
/// Button layout reference (bit positions) comes from Linux `xpad` and `medusalix/xone`.
pub fn parsed_to_dolphin(p: ParsedInput, cal: StickCalibration, cfg: BridgeConfig) -> DolphinState {
    let b = p.buttons;

    // GIP 0x20 button word layout (per Linux xpad / medusalix xone reference drivers):
    //   bit 2 Menu, bit 3 View, bits 4..7 A/B/X/Y, bits 8..11 d-pad U/D/L/R,
    //   bits 12..13 LB/RB, bits 14..15 LS/RS.
    let start = (b & 0x0004) != 0; // Menu  -> GameCube Start
    let view = (b & 0x0008) != 0; // View  -> GameCube Z
    let a = (b & 0x0010) != 0;
    let bb = (b & 0x0020) != 0;
    let x = (b & 0x0040) != 0;
    let y = (b & 0x0080) != 0;
    let d_up = (b & 0x0100) != 0;
    let d_down = (b & 0x0200) != 0;
    let d_left = (b & 0x0400) != 0;
    let d_right = (b & 0x0800) != 0;

    let z = view;

    // Convert to [-1,1], subtract learned centers, apply deadzone, then map to [0,1].
    let lx = clamp11(norm_i16_to_f1(p.lx) - cal.lx0);
    let ly = clamp11(norm_i16_to_f1(p.ly) - cal.ly0);
    let rx = clamp11(norm_i16_to_f1(p.rx) - cal.rx0);
    let ry = clamp11(norm_i16_to_f1(p.ry) - cal.ry0);

    let (lx, ly) = apply_radial_deadzone(lx, ly, cfg.deadzone);
    let (rx, ry) = apply_radial_deadzone(rx, ry, cfg.deadzone);

    let main_x = clamp01((lx + 1.0) * 0.5);
    let main_y = if cfg.y_invert {
        clamp01(((-ly) + 1.0) * 0.5)
    } else {
        clamp01(((ly) + 1.0) * 0.5)
    };
    let c_x = clamp01((rx + 1.0) * 0.5);
    let c_y = if cfg.y_invert {
        clamp01(((-ry) + 1.0) * 0.5)
    } else {
        clamp01(((ry) + 1.0) * 0.5)
    };

    let l = norm_trig10_to_01(p.lt10);
    let r = norm_trig10_to_01(p.rt10);

    DolphinState {
        a,
        b: bb,
        x,
        y,
        z,
        start,
        d_up,
        d_down,
        d_left,
        d_right,
        main_x,
        main_y,
        c_x,
        c_y,
        l,
        r,
    }
}

