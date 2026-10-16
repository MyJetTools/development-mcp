# Deep links — the URL vocabulary opens the app

New on the phone. The URL vocabulary the web client and the app share (topic `routing`) is the
app's deep-link vocabulary: a link mailed to a user opens on whatever device is in their hand — in
the app, when it is installed.

The one that matters most is a **one-time-login link** (`/auth?token=…` in the MT terminal), minted
by an admin console or an integration API and tapped on a phone.

- **iOS — Universal Links**: the `applinks:<host>` entitlement, and
  `/.well-known/apple-app-site-association` (no extension) served from the brand's host.
- **Android — App Links**: an `<intent-filter android:autoVerify="true">` for the host, and
  `/.well-known/assetlinks.json` served from it.
- **Both files are served by the web host of each brand** (`web-app-host`) — a server-side task
  before the first release. Apple's CDN can take up to a day to see a new file.
- **`go_router` takes the incoming link** — deep linking is on by default since Flutter 3.27 — and
  the route is the vocabulary's, so nothing else is needed in Dart.
- **A one-time-login route clears the previous session first**, before anything authorized can
  fire — on the web a `use_hook(clear_tokens)`, here the core input its screen's state sends in its
  constructor — then redeems the token.
- **A link to a path the app does not declare is a bug** — the routes guard test catches it before
  a user does.
