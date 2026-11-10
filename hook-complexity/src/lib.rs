// SPDX-License-Identifier: Apache-2.0
//! Bounded text classification through the host's invocation-bound child dispatch.
#![deny(unsafe_code)]
#![deny(missing_docs)]
mod config;
mod door;
mod routing;
/// The plugin's canonical name.
pub const NAME: &str = "busbar-hook-complexity";
/// Registry alias and policy word.
pub const ALIAS: &str = "complexity";
/// Linked and dropped-in builds share this entry.
pub use door::{door, MACRO_DOOR};
/// Linked registration entry.
pub mod linked {
    pub use crate::{door, MACRO_DOOR};
}
