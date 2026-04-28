# PowerA Xbox Controller Fix for Dolphin (macOS)

If you plugged in a wired PowerA Xbox-style controller into macOS and **Dolphin doesn’t see any inputs**, this is for you.
Some PowerA controllers show up on USB, but macOS does not interpret their controller data, so apps never receive button or stick events.

This project is a small background program that runs while Dolphin is open and:

- Reads the controller directly over USB
- Translates sticks/buttons/triggers into Dolphin's Pipe input protocol
- Sends that input to **Dolphin Emulator** using Dolphin’s built-in **Pipe** controller backend

No kernel extensions. No SIP changes. You typically run it with `sudo` so it can access the USB device.

## Will it work with my controller?

Right now, it is **confirmed for one specific model** (hard-coded Vendor ID / Product ID):

- **PowerA Xbox Series X Advantage Hall Effect Wired Controller**
  - **VID**: `0x20D6`
  - **PID**: `0x2079`

It **might** work for other *wired* PowerA Xbox-style controllers that use Microsoft's GIP (Game Input Protocol), but that is not guaranteed.
If your controller has a different VID/PID, support usually means: add the VID/PID and confirm the packet layout.

## Status

- **USB open + claim + init packet**: implemented
- **Input parsing**: implements a common GIP `0x20` input packet layout
- **Dolphin Pipe output**: implemented (PRESS/RELEASE + SET MAIN/C/L/R)
- **Auto payload offset detection**: implemented

## Quick start

1. **Run the program**

On macOS, accessing a vendor-specific USB controller usually requires root privileges:

```bash
make run
```

2. **Tell Dolphin to use the Pipe backend**

The program writes to this pipe (it will create the directory and FIFO automatically if missing):

- `~/Library/Application Support/Dolphin/Pipes/powera`

In Dolphin:

- Controllers → Standard Controller → Configure → Device dropdown: `Pipe/0/powera`

3. **Map controls inside Dolphin**

In Dolphin’s controller mapping UI, bind buttons/axes as you prefer. This program emits:

- Buttons: `A B X Y Z START D_UP D_DOWN D_LEFT D_RIGHT`
- Sticks: `SET MAIN x y` (left stick), `SET C x y` (right stick)
- Triggers: `SET L value`, `SET R value` (analog 0–1)

## Build (optional)

```bash
make build
```

If you want to build/run without `make`:

```bash
CARGO_TARGET_DIR=target cargo build --release
sudo ./target/release/xbox_controller_macos_gip
```

## Debugging / failure modes

- **USB claim fails**: likely permission/device-busy. Try unplug/replug, quit any software that might open it.
- **Init sends but no input arrives**: endpoints may be different; we auto-detect interrupt IN/OUT from descriptors.
- **Input arrives but parses wrong**: the program prints raw hex for the first few parse failures.
- **Pipe doesn’t connect**: Dolphin hasn’t opened it yet. Set Device to `Pipe/0/powera` and keep the config window open.

