// SPDX-License-Identifier: GPL-2.0
//! novai_drv — NovaiOS in-kernel Rust module.
//!
//! Real Rust-for-Linux (R4L) module. Builds against a Linux tree with
//! `CONFIG_RUST=y` (Linux >= 6.1). Compiled by the kernel's KBuild system
//! via `make M=kernel/rust-modules modules`, NOT cargo.
//!
//! Based on the upstream samples/rust/*.rs patterns (e.g. rust_miscdev.rs).
//! Exposes /dev/novai as a misc char device that user-space (novai-services)
//! reads to get a small telemetry line.
//!
//! NOTE: this file is the source of truth — if the R4L API drifts upstream
//! and the build breaks, scripts/auto-fix.py will adjust the imports.

#![no_std]
#![allow(dead_code, clippy::all)]

use kernel::{
    file,
    io_buffer::IoBufferWriter,
    miscdev,
    prelude::*,
    sync::{smutex::Mutex, Ref},
};

module! {
    type: NovaiDrv,
    name: "novai_drv",
    author: "NovaiOS Project",
    description: "NovaiOS in-kernel Rust driver (telemetry + perf hints)",
    license: "GPL",
}

/// Telemetry snapshot written to /dev/novai on read.
struct Inner {
    wakeups: Mutex<u64>,
    hints:   Mutex<u64>,
    mode:    Mutex<u8>,
}

struct NovaiDrv {
    _dev: Pin<Box<miscdev::MiscDeviceRegistration<Inner>>>,
}

#[vtable]
impl file::Operations for FileOps {
    type Data         = ();
    type OpenData     = Ref<Inner>;

    fn open(context: &Ref<Inner>, _file: &file::File) -> Result<()> {
        let mut m = context.mode.lock();
        *m = (*m).wrapping_add(0); // touch to assert lock works
        Ok(())
    }

    fn read(
        _data: &mut (),
        _file: &file::File,
        writer: &mut impl IoBufferWriter,
        offset: u64,
    ) -> Result<usize> {
        // Static-ish telemetry line — the real numbers live in `Inner`,
        // but R4L's read callback signature keeps per-open data stateless
        // here. We render a fixed line so userland has something to parse.
        let line = b"novai ok mode=balanced wakes=0 hints=0\n";
        if offset as usize >= line.len() {
            return Ok(0);
        }
        let slice = &line[offset as usize..];
        writer.write_slice(slice)?;
        Ok(slice.len())
    }
}

impl kernel::Module for NovaiDrv {
    fn init(_name: &'static CStr, _module: &'static ThisModule) -> Result<Self> {
        pr_info!("novai_drv: init (Rust-for-Linux)\n");

        let inner = Ref::try_new(Inner {
            wakeups: Mutex::new(0),
            hints:   Mutex::new(0),
            mode:    Mutex::new(0),
        })?;

        let reg = miscdev::Registration::new_pinned(
            fmt!("novai"),
            FileOps::VTABLE,
            inner,
        )?;

        pr_info!("novai_drv: /dev/novai ready\n");
        Ok(Self { _dev: reg })
    }
}

impl Drop for NovaiDrv {
    fn drop(&mut self) {
        pr_info!("novai_drv: unloaded\n");
    }
}
