/// Known controller VID/PIDs seeded from Linux kernel `xpad.c`.
///
/// Source of truth:
/// - Linux: `drivers/input/joystick/xpad.c` (Torvalds tree)
///
/// We keep the author's development device first for backwards-compatibility:
/// if multiple supported devices are plugged in, the first match wins.
pub const KNOWN_CONTROLLERS: &[(u16, u16, &str)] = &[
    (0x20D6, 0x2079, "PowerA Xbox Series X Advantage Hall Effect Wired"),
    (0x20D6, 0x2009, "PowerA Enhanced Wired Controller for Xbox Series X|S"),
    (0x20D6, 0x200E, "PowerA Spectra Infinity Enhanced Wired Controller"),
    (0x20D6, 0x2064, "PowerA Wired Controller for Xbox"),
    (0x20D6, 0x281F, "PowerA Wired Controller For Xbox 360"),
    (0x20D6, 0x2001, "BDA / PowerA Xbox Series X Wired Controller"),
    (0x20D6, 0x2003, "PowerA Xbox Series X Fusion Pro 2 Wired"),
    (
        0x20D6,
        0x2004,
        "PowerA Enhanced Wired Controller (Xbox Series X EnWired Pink Inline)",
    ),
    (0x0E6F, 0x0139, "PDP Afterglow Prismatic Wired Xbox One"),
    (0x0E6F, 0x013A, "PDP Xbox One Controller"),
    (0x0E6F, 0x0146, "PDP Rock Candy Wired for Xbox One"),
    (0x0E6F, 0x0161, "PDP Xbox One Controller"),
    (0x0E6F, 0x0162, "PDP Xbox One Controller"),
    (0x0E6F, 0x0163, "PDP Xbox One Controller"),
    (0x0E6F, 0x0164, "PDP Battlefield 1 Xbox One"),
    (0x0E6F, 0x0165, "PDP Titanfall 2 Xbox One"),
    (0x0F0D, 0x0067, "HORI Pad Pro Xbox One"),
    (0x0F0D, 0x0078, "HORI Real Arcade Pro V Kai Xbox One"),
    (0x24C6, 0x541A, "PowerA Xbox One Mini"),
    (0x24C6, 0x542A, "PowerA Xbox One Spectra"),
    (0x24C6, 0x543A, "PowerA Xbox One"),
    (0x24C6, 0x551A, "PowerA Fusion Pro Wired Xbox One"),
    (0x24C6, 0x561A, "PowerA Xbox One Cabled"),
    (0x24C6, 0x581A, "PowerA Enhanced Wired (3rd party)"),
    (0x045E, 0x02DD, "Microsoft Xbox One Controller (wired, post-2015)"),
    (0x045E, 0x02E3, "Microsoft Xbox One Elite Controller (wired)"),
    (0x045E, 0x02EA, "Microsoft Xbox One S Controller (wired)"),
    (
        0x045E,
        0x02FD,
        "Microsoft Xbox One S Controller (Bluetooth firmware, USB)",
    ),
    (0x045E, 0x0B00, "Microsoft Xbox One Elite Series 2 (wired)"),
    (0x045E, 0x0B12, "Microsoft Xbox Series X|S Controller (wired)"),
];

pub fn print_list() {
    for (vid, pid, name) in KNOWN_CONTROLLERS {
        println!("0x{vid:04X}:0x{pid:04X}  {name}");
    }
}

