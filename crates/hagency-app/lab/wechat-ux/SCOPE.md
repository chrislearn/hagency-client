# Agreed production scope

The user selected **Matrix messaging and the four native mobile tabs first**.
Broader WeChat parity is a separate workstream. This scope applies to production
implementation and live Palpo checks; it does not award or lower the visual
similarity gate.

| Surface | Production pass | Separate parity work |
| --- | --- | --- |
| Chats | Real joined rooms, direct/group conversations, drafts, sends, replies, edits, reactions, downloaded images, unread state, notifications and chat information | WeChat-specific social identity, voice/video services |
| Contacts | Matrix direct-room targets, directory/full-ID search, profiles, direct messaging and joined groups | Persistent social address book, friend requests, tags and phone-contact import |
| Discover | Real Matrix room discovery and spaces | Moments, scan workflows, mini programs and other service destinations |
| Me | Signed-in Matrix profile and existing account, general, privacy and about settings | Payments, wallet, social collections and WeChat account services |
| Runtime evidence | Isolated native client against real Palpo, recipient-confirmed effects, reconnect, responsive layout and soak receipts | Physical iOS/Android keyboards, permissions, platform push delivery and full localized release qualification |

Native file-picker uploads and additional settings screens retain their existing
Robrix implementations. Their complete mobile flow validation is still pending;
downloaded-image checks do not imply upload coverage.

The current reference pack contains 48 states and 22 journeys. Its unchanged full
parity gate requires 96 screen reviews and 44 localized journey receipts, plus
reviewed visual scores. Those requirements remain in the broader acceptance
workstream. The native production checks have separate receipts and must never
be presented as a 9/10 similarity result.

See [implementation](IMPLEMENTATION.md), [visual gaps](REVIEW.md) and
[validation summary](validation-summary.json) for current results and build IDs.

The user's subsequent request adds [web mini-app card sharing and Apple
PingFang typography](MINI-APPS.md). This extends the production work to user
HTTP/HTTPS pages embedded through Makepad/WebKit and shared through Matrix.
Tencent official accounts and the mini-program runtime remain separate.

The subsequent explicit implementation request adds the first
[Moments and File Transfer slice](MOMENTS.md), following
[ADR 0001](../../docs/adr/0001-matrix-moments.md). It does not include per-post
audiences, mutual-friend-only comments, or a passed visual-similarity gate.
