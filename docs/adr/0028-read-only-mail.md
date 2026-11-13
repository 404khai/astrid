# ADR 0028: Read-only mail from Apple Mail and Gmail

Date: 2026-11-11
Status: **Accepted 2026-11-11.** The maintainer answered "make all the decisions, yes for both" to the Gmail API option.
Related: [v2 gameplan](../plans/v2-gameplan.md) V2-03a; ADRs 0021 (exact foreground authority) and 0008/0009 (traces).

## Context

V2-03a asks Astrid to find "that email". The maintainer uses Apple Mail and also
Gmail in the browser, where no local copy exists.

## Decision

1. **Read-only only.** Two tools: `mail_search {source, query, limit ≤ 20}` and
   `mail_read {source, id}`. Astrid cannot send, reply, move, label, delete or
   mark messages, and no write scope is ever requested.
2. **Opt-in per invocation.** `astrid run --mail apple|gmail|all`. Each mail
   call shows one approval naming the source, the exact query or message id and
   the result limits. Approving a call allows that one call. Mail tools are
   never available without the flag.
3. **Apple Mail is local.** Search uses Spotlight (`mdfind`) restricted to
   `~/Library/Mail`, then parses the matching `.emlx` files for headers and a
   plain-text snippet. Nothing leaves the Mac except what the model is shown.
   macOS may require Full Disk Access for the terminal, or for whichever app
   runs Astrid. If Spotlight returns nothing, Astrid reports that and never
   claims the mailbox is empty.
4. **Gmail goes through the Gmail API, read-only.**
   - Sign-in uses OAuth 2.0 for installed apps, with PKCE and a loopback
     redirect on `127.0.0.1`, through `astrid mail gmail-login`. The only
     scope requested is `https://www.googleapis.com/auth/gmail.readonly`.
   - The OAuth client is one the user creates in Google Cloud. They save the
     downloaded file at `~/.config/astrid/gmail-client.json`.
   - The refresh token lives only in the **macOS Keychain** (service
     `dev.astrid.gmail`), written through `security -i` so the secret never
     appears in process arguments.
   - `astrid mail gmail-logout` deletes the Keychain item and revokes the
     token at Google.
   - Access tokens stay in memory only. Each run's mail requests go only to
     `oauth2.googleapis.com` and `gmail.googleapis.com`.
5. **Limits.**
   - **Search:** at most 20 results, returning sender, subject, date and a
     snippet of at most 300 characters.
   - **Read:** at most 16 KiB of plain text. HTML is reduced to text, and
     attachments are named but never fetched.
   - **Timeouts:** 10 s per request.
6. **Privacy.** Mail content goes to the model only through approved tool
   results. It is kept in the private session like other tool results and never
   in metadata traces. Message text cannot grant permissions; it is untrusted
   task data.

## Alternatives considered

- **Looking at Gmail in the browser (ADR 0027):** this already works today with
  `--look`, but it can't search mail you haven't opened.
- **IMAP with a Google app password:** the password gives full mailbox access,
  which is broader than search needs.

## Consequences

The user must create a Google Cloud OAuth client and sign in once. Any later
drafting or sending feature needs a separate ADR with confirm-before-send.

Cost of changing later: Medium.
