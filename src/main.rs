use std::sync::Arc;

use app::AppContext;

use crate::settings::SettingsReader;

mod app;
mod http;
mod mcp;
mod settings;

#[tokio::main]
async fn main() {
    let settings_reader = SettingsReader::new("~/.devops-mcp").await;
    let settings_reader = Arc::new(settings_reader);
    let app = AppContext::new(settings_reader).await;
    let app = Arc::new(app);

    crate::http::start(&app).await;
    app.app_states.wait_until_shutdown().await;
}
