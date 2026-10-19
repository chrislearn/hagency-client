# Desktop Contacts and Article editor

The wide desktop sidebar now provides two direct entries:

- **Contacts / 通讯录:** the people icon directly below Home. It opens the
  account-backed contact list, local filtering, New Friends lookup, group chats,
  and contact details. Opening a contact or group reveals its conversation.
- **Article editor / 文章编辑器:** the pencil icon below Moments. Select
  **Continue with Rinx**, then **Allow and open**, to reach the local article library.

Both tooltips follow the application language. Contacts uses the existing Matrix
contact page and account isolation; no sample contacts are added to the application.
The editor does not require a desktop Discover menu.

## Verification

On 2026-09-22, 14 native interaction checks completed using Makepad loopback
instrumentation and the real Metal backend with hidden windows. Every visibility
sample was zero. Checks cover contacts, filtering, profiles, existing DMs and
joined groups, three repeated DM/group switching cycles, desktop navigation,
article creation, native math/diagram preview, saving, reopening, and source
preservation after process restart.

The fixture server was offline. The initial log gate rejected expected read-receipt
and backwards-pagination connection failures after all UI assertions completed.
The [audited result](../article-editor/desktop-entry-evidence/result-audited.json)
records those errors separately and finds zero unexpected runtime errors. The
[original result](../article-editor/desktop-entry-evidence/result.json) is preserved.
Live directory search results were not verified in this offline run.

The run exposed a Makepad Metal lifetime bug: cached draw lists followed references
to dropped widgets. The fix skips freed/recycled IDs in texture uploads and rendering.
Its reproducible [patch and application instructions](../../tools/patches/README.md)
are included. Temporary backtrace instrumentation and an ineffective full-redraw
workaround were removed.

Build and translation checks passed (992 catalog entries, zero missing translations).
The tested signed executable has SHA-256 `e56006da2ef253ce62cca361ea5f5598ac24c9da22745c7615e482b825cb9bb3`.

## Native captures

These captures use the isolated synthetic account. Any offline banner refers to
the test server, not the user's matrix.org account.

[Contacts](../article-editor/desktop-entry-evidence/desktop-contacts.png) ·
[Contact details](../article-editor/desktop-entry-evidence/desktop-contact-profile.png) ·
[Direct chat](../article-editor/desktop-entry-evidence/desktop-contact-chat.png) ·
[Group chat](../article-editor/desktop-entry-evidence/desktop-group-chat.png) ·
[Editor button](../article-editor/desktop-entry-evidence/desktop-article-button.png) ·
[Article library](../article-editor/desktop-entry-evidence/desktop-article-library.png) ·
[Native preview](../article-editor/desktop-entry-evidence/desktop-article-preview.png)
