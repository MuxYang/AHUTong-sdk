#[cfg(feature = "network")]
pub mod data {
    pub mod api {
        pub mod adwmh;
        pub mod client;
        pub mod jwxt;
        #[cfg(feature = "domain-campus-card")]
        pub mod ycard;
    }
    #[cfg(feature = "domain-campus-card")]
    pub mod auth;
    pub mod crawler;
    pub mod model;
}

pub mod utils {
    #[cfg(feature = "domain-account")]
    pub mod des;
    pub mod parser;
}

#[cfg(all(not(target_arch = "wasm32"), feature = "legacy"))]
pub mod updater;

#[cfg(feature = "legacy")]
mod ffi;
#[cfg(feature = "legacy")]
mod persistence;

#[cfg(all(target_os = "android", feature = "legacy"))]
pub mod jni;

#[cfg(feature = "legacy")]
pub mod core;
#[cfg(all(
    not(target_arch = "wasm32"),
    feature = "network",
    not(feature = "legacy")
))]
pub mod plugin;
#[cfg(all(target_os = "android", not(feature = "legacy")))]
mod plugin_jni;
#[cfg(feature = "server")]
pub mod server;
#[cfg(all(not(target_arch = "wasm32"), feature = "storage-migration"))]
pub mod storage_migration;

#[cfg(all(target_arch = "wasm32", feature = "legacy"))]
pub mod wasm;
