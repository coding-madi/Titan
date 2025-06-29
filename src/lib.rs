pub mod api;
pub mod application;
pub mod config;
pub mod core;
pub mod infrastructure;
pub mod version;

#[allow(dead_code, unused_imports)] // Apply to the module
pub mod schema {
    include!("core/schema/mod.rs");
}
