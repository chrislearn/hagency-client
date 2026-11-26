# Remote Agent runtime demo

Hagency Client can route one owner's encrypted private Agent chat to an independent
[Hagency Agent Runtime Server](https://github.com/chrislearn/hagency-agent-runtime-server).
The execution device handles Matrix identity, decryption and encrypted replies.
The runtime host runs Codex CLI or Claude Code using its own provider login.

## Bind an Agent

1. Create a private Agent in Hagency Client, assign it to the current device and open
   its encrypted private chat. Prepare and verify its Matrix identity under **Service**.
   The Hagency host must support the personal encrypted inbox protocol. Set
   `personal_encrypted_delivery = "ciphertext_inbox"` on hosts that default to
   executor-device delivery.
2. Ask the runtime operator for a grant matching your Matrix user ID, Hagency
   Agent ID, provider and model. Use an HTTPS origin with a trusted certificate.
3. Stop the Agent service on the assigned device. Find the account directory
   containing `profile-identity.json` and `agent-local.sqlite` under
   `<client-data>/native-owner/<account>/owned-agent-owners/owner_<digest>/`.
   Create its `runtime-bindings/` directory with mode 700 and save the JSON below
   as `runtime-bindings/<agent-id>.json` with mode 600. The binding token is private.
4. Open **Service**, refresh, confirm the HTTPS origin and choose **Enable remote
   execution**. Send a text message in the Agent's encrypted private chat.

```json
{
  "endpoint": "https://runtime.example",
  "binding_id": "<binding UUID>",
  "token": "<binding token>",
  "provider": "codex",
  "model": "<model from grant>",
  "allow_encrypted_owner_direct": true
}
```

The consent field lets the assigned device send the current message as plaintext
over HTTPS after Matrix decryption. The runtime receives the message text and
its owner, Agent and room routing identifiers. The runtime service can retain
prompts and answers in its own database. This demo covers one message per run
and supports text in the owner's encrypted private chat. Shared rooms,
attachments, conversation history and token reservations are outside this flow.

## Stop or switch runtimes

**Disable remote execution** stops routing new requests and requests cancellation
of in-progress turns. The runtime service runs independently. To switch, stop
execution, replace the binding file and enable execution again. To return to
local execution, stop execution, remove the binding and start with the saved
local model. The Agent and its Matrix room keep the same IDs.

For an unconfirmed submission, find the original run ID, binding ID and endpoint
in the private `remote-runtime-runs/<agent-id>/` directory beside the ledger.
Query `GET /v1/runs/<run-id>` at that endpoint with the original binding token.
Keep the receipt until its outcome is confirmed. Client keeps the original
execution ID through submission and polling and keeps unknown receipts for
manual reconciliation.
