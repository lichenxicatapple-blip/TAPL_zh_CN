//! Compiled source for the Rust counterparts printed in the TAPL translation.
//!
//! Stable marker pairs delimit the exact regions extracted into the LaTeX
//! build.  The surrounding module structure and tests are intentionally not
//! printed, but ensure that every displayed fragment is real Rust code.

// These public functions are deliberately small, book-facing counterparts of
// the OCaml fragments. Requiring API documentation or must-use annotations
// would add material unrelated to the comparison printed in the book.
#![allow(
    dead_code,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::only_used_in_recursion
)]

pub mod chapter04;
pub mod chapter07;
pub mod chapter10;
pub mod chapter11;
pub mod chapter17;
