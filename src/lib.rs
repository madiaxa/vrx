//! VRX's GUI-independent document, playback, persistence, and export API.
#![forbid(unsafe_code)]
pub mod config;
pub mod countdown;
pub mod engine;
pub mod export;
pub mod import;
pub mod keybinds;
pub mod numerals;
pub mod persistence;
pub mod text;
pub mod version;

pub const TAGLINE: &str = "Read Rapidly";
pub const VERSION: &str = "v0.0.1";
pub type Result<T> = std::result::Result<T, String>;
