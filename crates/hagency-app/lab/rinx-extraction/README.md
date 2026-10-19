# Rinx extraction validation

Rinx starts from Robrix2's `wechat-ui` commit `16913e0c`, with its history intact.
The root package/executable is now `rinx`, the application name is Rinx, and the
bundle identity is `org.octosense.rinx`. App data is isolated by default; existing
Matrix event identifiers remain compatible.

Rinx uses Apache-2.0 with the original Robrix MIT notice preserved. README,
NOTICE, and the English/Chinese About pages acknowledge Robrix. Packaging includes
the Apache license and the inherited notices.

[validation.json](validation.json) records successful local build, unit-test,
translation and native UI checks. There are 188 passing app tests (two opt-in
live integration tests ignored) and 16 passing portable article-core tests.
The native article journey uses disposable Palpo accounts and private profiles;
credentials and raw input/log files are excluded from this report.

- [Rinx sign-in](evidence/login.png)
- [Chinese About page and Robrix attribution](evidence/about.png)
- [Native article HTML/CSS preview](evidence/article-preview.png)

The pre-extraction evidence under `lab/article-html-integration` remains a
historical Robrix snapshot. Mobile devices, external SSO accounts, signed
installers, and app-store publication were not exercised for this extraction.
