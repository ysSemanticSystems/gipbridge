use anyhow::{anyhow, bail, Context, Result};
use rusb::{Context as RusbContext, DeviceHandle, Direction, TransferType, UsbContext};

use crate::controllers::KNOWN_CONTROLLERS;

pub fn find_interrupt_endpoints(handle: &mut DeviceHandle<RusbContext>) -> Result<(u8, u8, u8)> {
    let dev = handle.device();
    let cfg = dev
        .config_descriptor(0)
        .or_else(|_| dev.active_config_descriptor())
        .context("config_descriptor")?;

    for interface in cfg.interfaces() {
        for interface_desc in interface.descriptors() {
            let iface = interface_desc.interface_number();

            let mut ep_in: Option<u8> = None;
            let mut ep_out: Option<u8> = None;
            for ep in interface_desc.endpoint_descriptors() {
                if ep.transfer_type() != TransferType::Interrupt {
                    continue;
                }
                match ep.direction() {
                    Direction::In if ep_in.is_none() => ep_in = Some(ep.address()),
                    Direction::Out if ep_out.is_none() => ep_out = Some(ep.address()),
                    _ => {}
                }
            }

            if let (Some(in_ep), Some(out_ep)) = (ep_in, ep_out) {
                if handle.kernel_driver_active(iface).unwrap_or(false) {
                    let _ = handle.detach_kernel_driver(iface);
                }
                handle
                    .claim_interface(iface)
                    .with_context(|| format!("claim_interface({iface})"))?;
                return Ok((iface, in_ep, out_ep));
            }
        }
    }

    bail!("could not find interface with BOTH interrupt IN and interrupt OUT endpoints")
}

pub fn open_controller(
    ctx: &RusbContext,
    override_vid_pid: Option<(u16, u16)>,
) -> Result<(DeviceHandle<RusbContext>, u16, u16, &'static str)> {
    if let Some((vid, pid)) = override_vid_pid {
        let handle = ctx
            .open_device_with_vid_pid(vid, pid)
            .ok_or_else(|| anyhow!("device not found (VID=0x{vid:04X}, PID=0x{pid:04X})"))?;

        let dev = handle.device();
        let cfg = dev.active_config_descriptor().ok().map(|c| c.number()).unwrap_or(1);
        let _ = handle.set_active_configuration(cfg);
        let _ = handle.set_auto_detach_kernel_driver(true);
        return Ok((handle, vid, pid, "Override device"));
    }

    let devices = ctx.devices().context("usb device list")?;
    for dev in devices.iter() {
        let dd = match dev.device_descriptor() {
            Ok(d) => d,
            Err(_) => continue,
        };
        let vid = dd.vendor_id();
        let pid = dd.product_id();
        if let Some((_, _, name)) = KNOWN_CONTROLLERS
            .iter()
            .find(|(kvid, kpid, _)| *kvid == vid && *kpid == pid)
        {
            let handle = match dev.open() {
                Ok(h) => h,
                Err(_) => continue,
            };
            let cfg = dev.active_config_descriptor().ok().map(|c| c.number()).unwrap_or(1);
            let _ = handle.set_active_configuration(cfg);
            let _ = handle.set_auto_detach_kernel_driver(true);
            return Ok((handle, vid, pid, *name));
        }
    }

    eprintln!("No known supported controller found.");
    eprintln!("Connected USB devices (VID:PID):");
    for dev in ctx.devices().context("usb device list")?.iter() {
        if let Ok(dd) = dev.device_descriptor() {
            eprintln!("  0x{:04X}:0x{:04X}", dd.vendor_id(), dd.product_id());
        }
    }
    eprintln!();
    eprintln!("Try: gipbridge --vid 0xVVVV --pid 0xPPPP");
    bail!("no supported controller connected")
}

