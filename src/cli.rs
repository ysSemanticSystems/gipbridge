use anyhow::{anyhow, bail, Result};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "gipbridge", about = None, long_about = None)]
pub struct Cli {
    /// Override VID (e.g. 0x20D6). If set, --pid must also be set.
    #[arg(long)]
    pub vid: Option<String>,
    /// Override PID (e.g. 0x2079). If set, --vid must also be set.
    #[arg(long)]
    pub pid: Option<String>,
    /// Pipe filename under ~/Library/Application Support/Dolphin/Pipes/ (default: powera)
    #[arg(long, default_value = "powera")]
    pub pipe_name: String,
    /// Do not invert stick Y axes (default: invert)
    #[arg(long)]
    pub no_y_invert: bool,
    /// Radial stick deadzone in [0.0, 0.5] (default: 0.12)
    #[arg(long, default_value_t = 0.12)]
    pub deadzone: f32,
    /// Print raw hex for every input packet received
    #[arg(long)]
    pub dump: bool,
    /// Print known supported controllers and exit.
    #[arg(long)]
    pub list: bool,
}

pub fn parse_u16_maybe_hex(s: &str) -> Result<u16> {
    let t = s.trim();
    let t = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    let is_hexish = t.chars().any(|c| matches!(c, 'a'..='f' | 'A'..='F'));
    let v = if is_hexish {
        u32::from_str_radix(t, 16).map_err(|e| anyhow!("invalid hex value '{s}': {e}"))?
    } else if t.chars().all(|c| c.is_ascii_digit()) {
        t.parse::<u32>()
            .map_err(|e| anyhow!("invalid decimal value '{s}': {e}"))?
    } else {
        u32::from_str_radix(t, 16).map_err(|e| anyhow!("invalid value '{s}': {e}"))?
    };
    u16::try_from(v).map_err(|_| anyhow!("value out of range for u16: {s}"))
}

pub fn parse_override_vid_pid(cli: &Cli) -> Result<Option<(u16, u16)>> {
    match (&cli.vid, &cli.pid) {
        (None, None) => Ok(None),
        (Some(_), None) | (None, Some(_)) => {
            bail!("--vid and --pid must be provided together");
        }
        (Some(vs), Some(ps)) => Ok(Some((parse_u16_maybe_hex(vs)?, parse_u16_maybe_hex(ps)?))),
    }
}

