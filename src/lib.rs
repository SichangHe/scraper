//! This is intended to serve as a binary crate.
//!
//! Please see README on
//! [crates.io](https://crates.io/crates/recursive_scraper)
//! or [GitHub](https://github.com/SichangHe/scraper) for more information.
pub mod config;
pub mod file;
pub mod io;
pub mod middle;
pub mod ring;
pub mod schedule;
pub mod state;
#[cfg(test)]
mod test;
pub mod urls;
