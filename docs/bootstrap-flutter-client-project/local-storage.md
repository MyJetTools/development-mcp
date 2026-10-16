# Local storage — records in the platform's preferences, the token in Keychain / Keystore

The twin of the Dioxus client's `local-storage` topic. The browser's two storages become three
places on the phone, by what is kept:

| What | Web | Phone |
|---|---|---|
| the session token | `localStorage` | Keychain (iOS) / Keystore-backed storage (Android), through `lib/native/storage/session.dart` |
| records — settings mirrors, selections, the last e-mail, … | `localStorage` | the platform's preferences, **under the same keys**, through `lib/native/storage/prefs.dart` |
| what lives for one visit | `sessionStorage` | the core's memory — a visit is the life of the process |
| what follows the user between devices | a server key-value store | the same store, through the same core code |

The keys and the shapes are the core's; Dart stores what the core says, as it says.

## prefs.dart — the records

```dart
// lib/native/storage/prefs.dart — the records the core keeps (the browser's localStorage)
final class Prefs {
  const Prefs(this._prefs);

  final SharedPreferencesAsync _prefs;

  /// Every record the core keeps, by the core's own keys — read once, at start
  Future<Map<String, String>> load() async {
    final all = await _prefs.getAll(allowList: core_api.storedKeys().toSet());
    return {
      for (final MapEntry(:key, :value) in all.entries)
        if (value is String) key: value,
    };
  }

  Future<void> setString(String key, String value) => _prefs.setString(key, value);

  Future<void> remove(String key) => _prefs.remove(key);
}
```

- **Read once, at start, handed to the core** (`startCore` — topic `routing`); the first frame is
  then drawn from the right state, never drawn wrong and corrected.
- **Every change comes back as `Store` / `Forget`** and is written here — the twin of the web's
  write-through, where the state method that changes a value writes storage in the same method.
  Nothing else writes these records.

## session.dart — the token

```dart
// lib/native/storage/session.dart — the ONE place the token is read or written
final class SessionStore {
  const SessionStore(this._secure);

  final FlutterSecureStorage _secure;

  static const _key = 'session_token';

  Future<String?> read() => _secure.read(key: _key);

  Future<void> write(String? token) =>
      token == null ? _secure.delete(key: _key) : _secure.write(key: _key, value: token);
}
```

- **The core holds the token in memory**; Dart reads it once, at start, and hands it over. When
  the core changes it — sign-in, a confirmation that finds nothing, logout — it emits
  `PersistSessionToken`, and the effect handler writes it here.
- **`IOSOptions(accessibility: KeychainAccessibility.first_unlock_this_device)`** — readable after
  the first unlock, never synced to another device: a session belongs to this phone.

## Two absences — never read as one

The dioxus-utils rule, kept:

- **No record** — the key is absent, or this build cannot parse the value: the ordinary case, the
  core starts from scratch for it.
- **No storage** — the store refuses a read or a write: a failure with its reason on screen,
  never read as "nothing is stored". An app that keeps its session there is a different program
  without it.

**The one place they meet is the token, on purpose.** `flutter_secure_storage` (11.x) wipes an
entry whose key can no longer be unwrapped (`resetOnError`, on by default since v10) — the token
then reads as absent and the user signs in again. For a session token that is the right outcome,
and nothing else is kept there.

## What the platforms add

- **The Keychain outlives the app.** iOS keeps Keychain items after an uninstall, while the
  preferences go with the app — so the first start after an install (no `installed` record in the
  preferences) deletes the token before anything reads it. Otherwise a reinstalled app signs in as
  whoever used it before.
- **Android's Auto Backup excludes the secure storage.** A restored copy cannot be unwrapped on the
  new device; it would be wiped there anyway.
- **Logout clears both halves** — the token, and the core's memory (including a server KV
  snapshot and its unconfirmed writes), so the next user's state is not overwritten by the
  previous one's.
