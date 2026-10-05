# Settings Pattern

```rust
// settings.rs
service_sdk::macros::use_settings!();  // ← ALWAYS first

#[derive(
    my_settings_reader::SettingsModel,
    AutoGenerateSettingsTraits,
    SdkSettingsTraits,
    Serialize, Deserialize, Debug, Clone,
)]
pub struct SettingsModel {
    pub seq_conn_string: String,
    pub my_telemetry: Option<String>,
    pub postgres_conn_string: String,
    pub users_grpc_url: String,   // ← {service_name}_grpc_url for each client
}

// GrpcClientSettings impl — always in settings.rs
impl GrpcClientSettings for SettingsReader {
    async fn get_grpc_url(&self, name: &'static str) -> GrpcUrl {
        if name == UsersGrpcClient::get_service_name() {
            return self.use_settings(|s| s.users_grpc_url.clone().into()).await;
        }
        // one if per client
        panic!("Unknown grpc service name: {}", name)  // ← REQUIRED at end
    }
}
```

**NEVER** read settings directly from fields — only via `use_settings(|s| ...)`.
**NEVER** cache settings values — `SettingsReader` auto-refreshes every 30 seconds.
**ALWAYS** end `get_grpc_url` with `panic!` — catches missing client registrations.
