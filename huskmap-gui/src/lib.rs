//! Freya desktop map. Talks only to huskmap-core.
//!
//! The view model, theme and icon tables compile without Skia so their tests run anywhere.
//! Enable `desktop` to build the window.

pub mod icons;
pub mod query;
pub mod theme;
pub mod view_model;

#[cfg(feature = "desktop")]
pub mod app;
#[cfg(feature = "desktop")]
mod components;
#[cfg(feature = "desktop")]
pub mod fonts;

pub fn run() {
    #[cfg(feature = "desktop")]
    app::launch_app();
    #[cfg(not(feature = "desktop"))]
    {
        eprintln!("{}", huskmap_core::copy::get().gui_missing);
    }
}

#[cfg(all(test, not(feature = "desktop")))]
mod tests {
    #[test]
    fn run_without_desktop_does_not_panic() {
        super::run();
    }
}
