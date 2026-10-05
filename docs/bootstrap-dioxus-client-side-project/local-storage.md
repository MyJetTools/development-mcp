# localStorage Helpers

```rust
// web/storage/session.rs
const SESSION_TOKEN_KEY: &str = "mt_session_token";
const REFRESH_TOKEN_KEY: &str = "mt_refresh_token";

fn get_local_storage() -> web_sys::Storage {
    web_sys::window()
        .expect("no window")
        .local_storage()
        .expect("no local storage")
        .expect("local storage is None")
}

pub fn save_tokens(session_token: &str, refresh_token: &str) {
    let storage = get_local_storage();
    storage.set_item(SESSION_TOKEN_KEY, session_token).expect("failed to save session token");
    storage.set_item(REFRESH_TOKEN_KEY, refresh_token).expect("failed to save refresh token");
}

pub fn get_session_token() -> Option<String> {
    get_local_storage()
        .get_item(SESSION_TOKEN_KEY)
        .expect("failed to read session token")
}

pub fn clear_tokens() {
    let storage = get_local_storage();
    let _ = storage.remove_item(SESSION_TOKEN_KEY);
    let _ = storage.remove_item(REFRESH_TOKEN_KEY);
}
```
