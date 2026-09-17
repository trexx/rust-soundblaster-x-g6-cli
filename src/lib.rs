//! Library behind the `g6-cli` and `g6-watch` binaries: HID wire format (`spec`), persisted
//! state (`model`, `state`), the device layer (`device`), the high-level operations (`api`),
//! the connect watcher (`watch`) and logon registration (`autostart`).

pub mod api;
pub mod autostart;
pub mod cli;
pub mod device;
pub mod model;
pub mod spec;
pub mod state;
pub mod watch;
