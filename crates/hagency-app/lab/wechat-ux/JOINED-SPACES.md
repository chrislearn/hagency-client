# Joined Spaces in Chats

Chats now has an **All Chats / Spaces** switch below the mobile search bar.
All Chats is the default, flat list of direct and group conversations. On a
wide desktop window the same switch sits in the conversation sidebar under
the global search bar. The legacy desktop Space rail remains available.

Spaces shows joined Space names, disclosure chevrons, nested joined Spaces,
and joined conversation rooms. Selecting a conversation uses the normal room
navigation. Back preserves the tree's expansion state. **Browse rooms** opens
the existing Space lobby, where unjoined children can be inspected and joined.
The tree does not join a room automatically.

Discover now has **Explore Groups & Spaces**, opening the existing directory
and address search. The mobile joined-Spaces avatar strip is hidden; it no
longer appears below Discover or room search results.

## Data and behavior

- Uses the signed-in SDK client's joined memberships and locally synced
  `m.space.child` state. Removed/redacted links and rooms not joined are omitted.
- Refreshes local membership and unread data every three seconds while the
  Spaces view is active. It does not send the search text to the server.
- Search matches room/Space names and IDs, retains ancestor context, and opens
  matching paths without changing the saved expansion state.
- All Chats shows each room once. A shared room can appear under each of its
  Spaces, but each Space's unread total counts descendant rooms only once.
- Hidden chats are omitted from the normal tree and can be found by search.
- Cycles are guarded; disconnected/cyclic Space components remain reachable.
- Account changes discard transient data. Expansion state lasts for the app
  session, including adaptive layout changes; it is not persisted to disk.

## Validation

`cargo test --locked --features agent_chat --lib` passes 160 tests, including
graph cycles, multiple parents, unread deduplication, CJK name search, hidden
chats, unjoined children, and collapse behavior.

`python3 tools/wechat-ux/live/native_joined_spaces.py` uses only isolated Palpo
fixture accounts at 375×812 and 1050×800. Its screenshots, native input traces,
binary hashes, and final report are under the ignored
`evidence/live/joined-spaces/` directory. It checks the default flat list,
expansion, room/back navigation, search, collapse, Space lobby navigation,
Discover separation, flat-list deduplication, live child updates, and desktop
navigation. All ten native checks passed with zero native UI errors. This is
functional validation, not a new visual-similarity score.
