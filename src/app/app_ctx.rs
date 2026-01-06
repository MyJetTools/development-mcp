use std::sync::Arc;

use rust_extensions::AppStates;

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub app_states: Arc<AppStates>,
}

impl AppContext {
    pub async fn new() -> Self {
        AppContext {
            app_states: Arc::new(AppStates::create_initialized()),
        }
    }
}
