# SPDX-License-Identifier: GPL-2.0
# NovaiOS kernel Rust module — KBuild file
#
# Built via:  make -C /path/to/linux M=$(pwd) modules
#
# The kernel build system supplies:
#   - rustc flags via KBUILD_RUSTFLAGS
#   - the `kernel` crate (bindgen-generated bindings + safe abstractions)
#   - core / alloc via --extern
#
obj-m := novai_drv.o

# R4L links Rust sources via `rust_project.json`. The kernel makefile expects
# the crate name to match `obj-m` minus `.o`, with a `--extern` alias.
novai_drv-objs := $(src)/target/none/release/libnovai_drv.rlib
