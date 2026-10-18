//! # gauss-trainer
//!
//! A terminal trainer for the Gaussian algorithm (Gauss–Jordan elimination).
//! The student chooses every elementary row operation; the computer does the
//! arithmetic exactly, in rational numbers.
//!
//! The crate is a library with a thin binary on top. The core modules
//! ([`rational`], [`matrix`], [`ops`], [`echelon`], [`hint`], [`check`],
//! [`generator`], [`parse`]) perform no I/O and know nothing about the user
//! interface.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::float_arithmetic)]
#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

pub mod check;
pub mod echelon;
pub mod generator;
pub mod hint;
pub mod matrix;
pub mod notation;
pub mod ops;
pub mod rational;
