//! Native Linux host for the Melee decompilation.

#[cfg(feature = "window")]
pub mod app;
pub mod dvd;
pub mod ffi;
pub mod gx;
pub mod os;
pub mod pad;
#[cfg(feature = "window")]
pub mod renderer;

#[cfg(feature = "window")]
pub use app::HostApp;
pub use gx::GxState;
pub use pad::{PadStatus, PadSystem};
