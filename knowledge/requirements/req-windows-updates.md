---
kind: requirement
id: REQ-WINDOWS-UPDATES
title: Windows desktop automatic updates without interrupting Agent tasks
status: Accepted
---

The operator authorized the Windows first release of automatic checks, background
downloads and a Restart and Install action, on a new branch with a pull request.
Running Agent tasks postpone installation. A user can cancel that wait. Updates
must verify both the release manifest and the full NSIS package, preserve local
accounts and data, and complete normal shutdown before installation. Hosted apps
and other operating systems do not install updates through this Windows feature.
Do not publish a release or provision production signing credentials as part of
this pull request.
