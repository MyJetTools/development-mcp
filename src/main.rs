use std::sync::Arc;

use app::AppContext;

mod app;
mod http;
mod mcp;

#[tokio::main]
async fn main() {
    let app = AppContext::new().await;
    let app = Arc::new(app);

    crate::http::start(&app).await;
    app.app_states.wait_until_shutdown().await;
}
