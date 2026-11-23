//! Doc IR and printer. This crate has no language knowledge.
//!
//! A [`Doc`] can only be built through [`builder`], which emits no gap before a
//! glued token and emits every comment together with the token it belongs to.
//!
//! M0 ships the IR, the builder and identity printing. Full Wadler-style
//! printing arrives in M1.

pub mod builder;
mod ir;
mod printer;

pub use ir::{AlignItem, AlignRow, Doc};
pub use printer::{PrintOptions, print, print_bounded};
