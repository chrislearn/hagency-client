[English](privacy.md) | [简体中文](privacy.zh-CN.md)

# Hagency data and privacy

This describes the desktop application's data flows. Your Hagency server,
Pasion identity provider and configured model services have their own policies.

## Accounts and local storage

Hagency signs in to your selected Hagency server through Pasion in your browser.
The Matrix SDK maintains the chat session. Account and message data sent to the
server is governed by that server's policies and Room membership.

The app stores session information, encryption state, Room caches, settings,
article drafts and selected article assets in its local profile. Agent profiles,
workspaces, usage ledgers and recovery state are also stored locally and scoped
to the authenticated account. Switching accounts stops the previous account's
local Agent runtime; it does not erase that account's stored data.

`HAGENCY_APP_DATA_DIR` selects an absolute application data directory.
`HAGENCY_DESKTOP_DATA_DIR`, `RINX_DATA_DIR` and `ROBRIX_DATA_DIR` remain accepted
aliases. Clearing the event cache reloads Room events from the server; it does
not delete server messages or erase the entire application profile.

## Agents and configured services

Running an Agent can send its Room conversation, instructions and permitted file
content to the model/runtime service selected for that Agent. Enabled web search,
tools and speech services may contact their configured services. Review those
providers' policies and your Agent's resource and permission settings.

Human Matrix chat supports encryption. Agent execution in encrypted Rooms is
currently deferred; do not treat an Agent conversation as encrypted by default.

## Mini apps, articles and external links

Mini-app authorization is tied to the signed-in Matrix user and the requested
capabilities. The article mini app receives scoped host operations, not the
account password or a raw Matrix access token. Publishing or sharing an article
sends its content to the selected Matrix conversation.

Native HTML/CSS article preview uses the app's authorized article assets. Other
web mini apps and links you choose to open may contact their own sites. Optional
Hagency integrations communicate with the services configured for that workflow.

## Diagnostics and support

Logs and exported diagnostics can contain account or room identifiers. Review
them before sharing them in a public issue, and remove credentials, private
conversation content and personal file paths. See [support](support.md) for the
current feedback channel. Technical details of mini-app grants
are documented in [ADR 0002](adr/0002-octoscript-mini-app-authority.md).
