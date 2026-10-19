# Native comparison findings

The native port has **not passed 9/10 similarity**. The public Makepad WeChat
mock and generated atlas are design references, not captures of a specified
Tencent WeChat release. The local [comparison gallery](evidence/live/review.html)
shows the current five principal views side by side without assigning a score.

The implementation now has the four mobile roots, the combined chat list,
square avatars, unread badges, green selected tabs, incoming/outgoing bubbles,
native composition, contacts, profiles, joined groups and mobile settings entry.
These controls call Robrix's existing Matrix handlers. Palpo confirms sends,
reply relationships, edits, reactions and room notification changes; image
fixtures download and open in the native viewer. See the run-specific results
in [validation-summary.json](validation-summary.json).

Observed work remaining before visual acceptance:

| Area | Current difference | Required work |
| --- | --- | --- |
| Discover and Me | Matrix destinations cover fewer rows than the reference | Broader service destinations are separate parity work under the agreed [scope](SCOPE.md) |
| Conversation details | Chat Info passes its native checks; deeper settings retain existing styling | Review reference alignment and port remaining settings in the detailed parity pass |
| Message metadata | Edited labels fit; compact quotes now sit below bubbles | Long-quote expansion/collapse and jump-to-original pass; reaction spacing still needs visual review |
| Contacts | Direct-room targets provide the agreed Matrix contact surface | Persistent contact book, friend requests and tags remain separate parity work |
| Localization | New root labels are English; CJK IME has not been exercised | Add Chinese copy and real-device composition/keyboard captures |
| Evidence normalization | Reference and candidate have different fixture content and window chrome | Freeze matching viewports, safe areas, locales and fixture data before grading |
| Coverage | Focused runs do not cover the complete catalog | Supply all 96 screen reviews and 44 localized journey receipts required by the unchanged gate |

The two 30-minute soaks, responsive-window checks and reconnect test establish
only their recorded runtime behavior. They do not establish design similarity,
physical-device behavior, voice/call support, Moments, payments or mini programs.
Sample imagery is uploaded only to the isolated test accounts and is not bundled
as production artwork.
