use std::sync::Arc;

use app::AppContext;

mod app;
mod glibc_compat;
mod http;
mod mcp;
mod rag;
mod timers;

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

// Read by jemalloc at init on Linux (unprefixed build); the MALLOC_CONF env var
// still overrides. background_thread + short decay return freed pages to the OS
// within ~5s, which is what matters here: loading, dropping and reloading ONNX
// models leaves large holes that the system allocator keeps to itself.
#[allow(non_upper_case_globals)]
#[export_name = "malloc_conf"]
pub static malloc_conf: &[u8] = b"background_thread:true,dirty_decay_ms:5000,muzzy_decay_ms:5000\0";

#[tokio::main]
async fn main() {
    let app = AppContext::new().await;
    let app = Arc::new(app);

    crate::http::start(&app).await;
    app.app_states.wait_until_shutdown().await;
}
