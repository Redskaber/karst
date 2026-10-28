//! @path: karst/crates/karst_vm/lib.rs
//! @author: redskaber
//! @datetime: 2026-10-28
//! @discription: karst::crates::karst_vm
//!
//! bytecode virtual machine

mod builtin;
mod error;
mod frame;
mod io;
mod value;
mod vm;

pub use builtin::Builtin;
