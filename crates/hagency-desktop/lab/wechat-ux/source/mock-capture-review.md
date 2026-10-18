# Public Makepad WeChat mock: capture review

These screenshots were captured from <https://wasm.robius.rs/makepad_wechat/> at
400×800 logical pixels, device scale 2, giving **800×1600 stored pixels**. Capture
uses Chromium's SwiftShader WebGL renderer and actual pointer input to the native
Makepad WASM canvas. It does not render a recreated HTML imitation.

The immutable [v2 receipt](mock-captures-v2/capture-receipt.json) includes fetched
WASM hashes, browser version, time, viewport, input sequences and screenshot
hashes. The published deployment's source revision is unknown. Do not conflate it
with the separately inspected Git checkout or with a shipping WeChat version.

Visual inspection of the captures confirmed these screen identities:

| Capture | Observed content and limitation |
| --- | --- |
| [Chats](mock-captures-v2/chats.png) | Chinese title, English preview fixtures, avatars, timestamps, four tabs |
| [Plus menu](mock-captures-v2/chats-menu.png) | Add Contact, New Chat, Scan, Money; menu display does not prove those services work |
| [Add Contact](mock-captures-v2/add-contact-candidate.png) | Correct Add Contact route, input and shortcut rows; file retains its original candidate name |
| [Contacts](mock-captures-v2/contacts.png) | Shortcut rows and alphabet headings; mock grouping order is not a production-quality alphabetical specification |
| [Discover](mock-captures-v2/discover.png) | Moments, Scan, Shake, Search, People Nearby, Mini Programs |
| [Moments](mock-captures-v2/moments.png) | Cover and feed posts; no proof of publishing, comments, ACL or persistence |
| [Me](mock-captures-v2/me.png) | Profile section and Favorites/Posts/Stickers/Settings rows |
| [My Profile](mock-captures-v2/profile-edit.png) | Correct pushed profile route, photo/name/ID/QR/info/settings rows |
| [Conversation](mock-captures-v2/conversation.png) | Incoming/outgoing bubbles and composer, repeated synthetic CJK content |

The initial [v1 collection](mock-captures-v1/capture-receipt.json) is retained.
Its `profile-candidate.png` did **not** reach My Profile: tapping the name area
kept the Me screen open. Source inspection showed that only the QR/chevron region
was clickable. V2 used that region and reached the correct route. This failed
attempt is not silently promoted into a successful navigation test.

These are useful references for native widget layout and transitions. Their
oversaturated bubble green, placeholder content, nonuniform alphabetical groups,
limited interactions and mixed language are properties of the mock, not asserted
WeChat requirements. The generated atlas is a separate proposed Robrix design.
Compare both references when authoring components, then freeze one agreed target
per screen before scoring. Never select whichever reference yields the higher score.

No Robrix screenshot was compared here, and no 9/10 review was issued. Source
screen identity review is separate from native Robrix similarity acceptance.

Upstream project: [project-robius/makepad_wechat](https://github.com/project-robius/makepad_wechat).
Its [Apache-2.0 license](MAKEPAD_WECHAT_LICENSE.txt) is retained. Screenshots are
reference evidence, not newly licensed production artwork extracted from the demo.
