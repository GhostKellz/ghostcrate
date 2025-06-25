pub mod models;
pub mod db;
pub mod auth;
pub mod web;
pub mod storage;

use leptos::*;
use wasm_bindgen::prelude::wasm_bindgen;

#[cfg(feature = "ssr")]
use sqlx::SqlitePool;

#[cfg(feature = "ssr")]
#[derive(Clone)]
pub struct AppState {
    pub pool: SqlitePool,
    pub auth_config: auth::AuthConfig,
    pub storage: storage::Storage,
}

#[wasm_bindgen]
pub fn hydrate() {
    use crate::web::App;
    console_error_panic_hook::set_once();
    leptos::mount_to_body(App);
}