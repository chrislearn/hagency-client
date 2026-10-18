# Explore Rooms and back navigation

Explore has a fixed back arrow on mobile and desktop. It returns from a room
preview to its search results, then to the tab that opened Explore. Escape also
works while the search field has focus. Pending responses cannot reopen a page
after leaving it or overwrite a later search.

Enter an English or Chinese room name (including a partial name) and press Search
or Enter. Robrix searches the signed-in homeserver's public directory and cached
joined room names/aliases. It does not require a matrix.org URL. Private unlisted
rooms that you have not joined still require an alias, ID, link or invitation.
Full Matrix addresses retain their preview and join/knock confirmation behavior.
Directory pages have a More results button when the server provides a cursor.

Two-finger rightward trackpad navigation is handled through Makepad's phased
scroll events and the existing Back event. It reads the macOS Natural Scrolling setting at gesture start and
normalizes the direction, so rightward movement means Back with either setting. The gesture commits on lift-off after
72 points of predominantly horizontal movement. Vertical scrolling, small moves,
mouse wheels, leftward gestures, cancellation, and momentum do not navigate. It
also routes back through mobile chats, contact details, and Settings pages.

Validation:

- `cargo test --locked --features agent_chat --lib`: 146 tests, including query
  classification/routing and gesture direction, threshold, cancellation, vertical
  rejection and momentum suppression.
- `python3 tools/wechat-ux/live/native_explore.py`: 11 checks against isolated
  Palpo fixture accounts. Covers back/escape, English/Chinese partial names,
  private joined rooms, empty results, malformed URLs, full aliases, preview
  return, replacement queries and unchanged room membership.
- Native search/back layout checks also passed at 320×640 and 1024×768.
- Native screenshots and receipts are in ignored
  `evidence/live/explore-navigation/`; no personal account is automated.

Physical trackpad verification remains outstanding: Makepad's remote bridge
injects phase-less mouse-wheel events, and the external Cocoa event sender did
not deliver scroll events to the test window. Gesture unit tests do not establish
end-to-end hardware validation. Physical macOS trackpad and iOS
gestures and the original 9/10 visual similarity gate are not validated here.
