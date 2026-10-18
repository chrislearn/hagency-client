# Room search and shared attachments

Open a chat, tap **···**, then choose **Search Chat History** or **Shared Attachments**. These tools appear above the member list, including in large groups. The room-list context menu offers the same two entries in desktop mode.

Search matches message text, sender names, and attachment filenames, case-insensitively, including Chinese text. Results belong to the selected room and appear newest first. Select a result to read it, then choose **View in Chat** to return to its original message. Search opened from a thread covers the main room.

The filters are **All**, **Media** (photos and videos), **Files**, and **Audio**. Photos have thumbnails and a larger preview. Attachment details show the filename and size and use Robrix's existing **Download** and **Share** flows. Video/audio playback is not added by this change; their original messages and downloadable files remain available.

History is fetched with the pinned Matrix SDK's `Room::messages`, which attempts decryption with the current account's available keys. No plaintext search term is sent to the homeserver. Opening loads the most recent 100 events; **Search**/Enter refreshes from the newest history and checks up to 1,000 events per batch. **Load Older** continues from the saved pagination token; **Stop** cancels a running batch. Status text distinguishes incomplete searches from the end of available history. Missing encrypted messages are counted explicitly. Restore the required encryption keys and search again to retry them.

The transient index applies known edits only from the original sender and excludes redacted messages and the room's local Delete Chat history cutoff. It is refreshed by Search/Enter or reopening the page. Request IDs and account ownership guard asynchronous results; closing or switching accounts cancels pending reads and clears the index. It is not a persistent, whole-account search index.

Implementation: `src/home/room_history.rs`, Chat Info and room-context-menu entry points, the app modal, and mobile/desktop navigation back to the selected room event.

Validation runner: `python3 tools/wechat-ux/live/native_room_history.py`, using isolated Palpo accounts, a separate test profile, and native instrumentation on port 8299. `ROBRIX_HISTORY_REUSE=1` reuses its seeded history for subsequent runs. The fixture copies the stopped forwarding-test profile to preserve that test device's encryption keys. Evidence stays local under `lab/wechat-ux/evidence/live/room-history/`. Native OS save/share dialogs and physical iOS/trackpad behavior require separate manual checks.

Verified on macOS: the build and all 157 library tests passed. Nine core native checks passed for older-page search, returning to the original message, edited/redacted content, room isolation, photo details, filename matching, audio filtering, encrypted history, and the local history-clear cutoff. After adding original-resolution detail loading, four focused checks passed for desktop context-menu search/jump, preserving a mobile composer draft, comparing photo-preview pixels against the original fixture (mean RGB error 0.44/255), and Escape navigation. Both native runs recorded zero native UI errors. Receipts: `native-room-history.json` and `native-room-history-navigation.json`; their immutable run traces include build hashes.

The focused runner is `python3 tools/wechat-ux/live/native_room_history_navigation.py`; it expects the public Makepad WeChat reference checkout at `/tmp/robrix-wechat-reference` for its original-photo comparison. These are functional checks and do not award a WeChat similarity score.
