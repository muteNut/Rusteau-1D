//! # rusteau-core
//! A Rust port of the HYDRUS-1D numerical engine for one-dimensional variably
//! saturated water flow, heat and solute transport.

// HYDRUS uses truncated values of π (3.1415 in SOLUTE.FOR, 3.14159 in WATFLOW.FOR); they are kept for identical results.
#![allow(clippy::approx_constant)]

pub mod atm;
pub mod error;
pub mod examples;
pub mod grid;
pub mod heat;
pub mod lenhard;
pub mod material;
pub mod meteo;
pub mod model;
pub mod output;
pub mod report;
pub mod root;
pub mod sim;
pub mod solute;
pub mod units;
pub mod water;
pub mod vapor;
pub mod inverse;

pub use error::HydrusError;
pub use model::*;
pub use output::Results;
pub use sim::{Simulation, StepStatus};
pub use grid::generate_depths;
pub use units::convert_units;
