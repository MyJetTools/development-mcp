use std::sync::Arc;

use rust_extensions::AppStates;

use crate::settings::SettingsReader;

pub const APP_VERSION: &'static str = env!("CARGO_PKG_VERSION");
pub const APP_NAME: &'static str = env!("CARGO_PKG_NAME");

pub struct AppContext {
    pub app_states: Arc<AppStates>,
    pub settings_reader: Arc<SettingsReader>,
}

impl AppContext {
    pub async fn new(settings_reader: Arc<SettingsReader>) -> Self {
        AppContext {
            app_states: Arc::new(AppStates::create_initialized()),
            settings_reader,
        }
    }
}
