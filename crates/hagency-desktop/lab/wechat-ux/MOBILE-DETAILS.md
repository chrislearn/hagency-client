# Mobile Settings and profile details

The mobile layout now uses native Makepad grouped white rows, inset separators,
gray page backgrounds, centered headers with back navigation, right-aligned
values, square avatars, and green switches. Geometry follows the checked-out
`makepad_wechat/src/profile/my_profile_screen.rs` and its recorded
`source/mock-captures-v2/profile-edit.png` reference. Desktop Settings and its
profile pane retain their existing layout.

| Entry | Native page / behavior |
| --- | --- |
| Me → profile | Personal Information, separate from Settings; back returns to Me |
| Personal Information → Name | Dedicated editor; Save waits for Matrix success; failure retains the draft; back cancels |
| Personal Information → Profile Photo | Larger photo, existing validated image picker/upload flow, existing delete confirmation |
| Personal Information → Matrix ID | Copies the real Matrix ID |
| Settings → Account and Security | Real account, homeserver and device identifiers, device verification, homeserver account-management discovery |
| Settings → General | Hardware Enter preference, display zoom and chat-image size |
| Settings → Privacy | Public/private receipts, automatic/manual marking, receipt visibility, blocked users with confirmed unblock |
| Settings → About / Help | Existing version, website, privacy policy, source and feedback links |
| Settings → Log Out | Existing logout confirmation and controller |
| Contacts → person | Horizontal avatar/name/ID card, profile link, Messages, confirmed block/unblock |
| Chat → avatar | Full-width mobile detail page, profile link, room membership/role, receipt jump, Messages and confirmed block/unblock |

All profile data and writes use the existing Matrix SDK/controller path. A Matrix
ID is labeled as such. Unsupported WeChat account attributes and services are
not fabricated. Advanced desktop preferences remain available in the wide
layout; these mobile pages expose the common messaging preferences.

Run the isolated Palpo regression with:

```sh
cargo build --locked --features agent_chat
python3 tools/wechat-ux/live/native_mobile_details.py
```

It uses only the ignored test fixture in `evidence/live/fixture.json`, creates its
own profile, hides its window, and records input/captures in
`evidence/live/mobile-details/native-runs/`. The name-edit check verifies the
server result and restores the fixture name. Local preference checks inspect
saved state and restore their fixture defaults. The visible personal Matrix
account is not instrumented.

Functional passes and reviewed screenshots do not establish a 9/10 WeChat
similarity score. An independent visual evaluation and iOS device validation
remain separate requirements.

The pinned Makepad Metal screenshot path asserts if asked to blit a window while
its DPI override is non-default. The zoom regression therefore checks native
widget state at 110%, restores 100%, then captures. The normal zoom UI remains
operational; this does not validate screenshots at other zoom levels.

Validation for this pass: 139 Rust library tests passed; eight native Palpo
behavior checks passed at 375 × 812, including name save/cancel, confirmation
flows, profile navigation, closing contact details back to the same chat, and
preference persistence. Compact 320 × 640 and 406 × 900 layouts were also
captured and reviewed. Results and per-run binary hashes remain in the ignored
local evidence directory. The 9/10 visual gate has not been claimed.
The existing desktop Settings layout was also opened and captured at 1024 × 768.
The updated normal macOS app was relaunched with its existing session restored
and a visible window; its binary matches the final test build.
