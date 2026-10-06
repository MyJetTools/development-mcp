# localStorage Helpers

```rust
// web/storage/session.rs
use dioxus_utils::js::LOCAL_STORAGE;

const SESSION_TOKEN_KEY: &str = "mt_session_token";
const REFRESH_TOKEN_KEY: &str = "mt_refresh_token";

pub fn save_tokens(session_token: &str, refresh_token: &str) {
    LOCAL_STORAGE.set(SESSION_TOKEN_KEY, session_token);
    LOCAL_STORAGE.set(REFRESH_TOKEN_KEY, refresh_token);
}

pub fn get_session_token() -> Option<String> {
    LOCAL_STORAGE.get(SESSION_TOKEN_KEY)
}

pub fn clear_tokens() {
    LOCAL_STORAGE.delete(SESSION_TOKEN_KEY);
    LOCAL_STORAGE.delete(REFRESH_TOKEN_KEY);
}
```

Storage is accessed only through `dioxus_utils::js::LOCAL_STORAGE` / `SESSION_STORAGE` — the project has no storage accessor of its own and never takes the storage from `web_sys::window()`. There are **two different absences**, never read one as the other. **No token** — the key is absent: `get_session_token()` answers `None`, the ordinary "not signed in" case. **No storage** — it cannot be obtained (no `window`, the browser returns `null` or denies access), or the browser refuses a read, a write or a delete (e.g. the quota is exceeded): `dioxus-utils` panics, there is no `Option` or `Result` to check. An app that keeps its session in storage is a different program without it, so a failure is never read as "nothing is stored" — that would be a silent sign-out. With `dioxus_utils::set_panic_hook()` in `main()` the reason is in the browser console.

For screen state kept in browser storage — a state initialised from `sessionStorage` / `localStorage`, write-through, navigating with data — see **dioxus-design-patterns** (`get_dioxus_design_patterns`) §18.
