//! Admin dashboard (HTTP API + minimal HTML).

pub mod api;
pub mod html;
pub mod server;

pub use server::DashboardServer;
