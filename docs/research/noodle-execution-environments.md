# Noodle Computer, Browser and Hub: complementary execution research

Date: 2026-10-10
Question: Can Astrid reuse computer/browser infrastructure while retaining its
own orchestration, authority, state and inference boundaries?
Disposition: Research only; no integration, installation, execution or outreach

## Sources and confidence

The relevant project is [`pdparchitect/noodle`](https://github.com/pdparchitect/noodle).
Inspected upstream main at **`cb26ecf632e13ceda316d8b49ce8e0246a1dd70a`** through
GitHub tree metadata, READMEs, package manifests, security documentation and the
source files linked below. Source was fetched to temporary research storage only;
no upstream code was vendored into Astrid or executed.

Confirmed means documented or observed in code at this commit, not independently
verified deployment/security. Public CLI/MCP entry points exist for Computer and
Browser; their presence does not establish a long-term compatibility commitment.
The ComputerBridge/BrowserBridge/HubLink schemas have version checks, but internal
versioning is not a third-party API stability guarantee.

GitHub release metadata showed `computer-latest`, `browser-latest`, and `hub-latest`
with arm64 macOS DMG/ZIP assets, published respectively 2026-09-10, 2026-09-18 and
2026-09-24. Browser's component README still says a first release is forthcoming;
the root README and release metadata contradict that older sentence. Do not use
it to claim Browser is unreleased, or assume latest source equals the shipped CLI.
[Computer release](https://github.com/pdparchitect/noodle/releases/tag/computer-latest),
[Browser release](https://github.com/pdparchitect/noodle/releases/tag/browser-latest),
[Hub release](https://github.com/pdparchitect/noodle/releases/tag/hub-latest)

## Licensing and runtime implications

The repository uses Apache-2.0 and includes NOTICE. Reusing/distributing project
code requires preserving the license and applicable notices, identifying modified
files and carrying relevant NOTICE attribution. This licenses project code, not
trademarks or every dependency/guest operating system. A future bundled
distribution must inspect transitive software/image licenses separately; running
an independently installed companion avoids copying its stack into Astrid.
[Pinned LICENSE](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/LICENSE),
[NOTICE](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/NOTICE)

The apps are Swift/macOS projects, not Rust libraries or portable hosted services.
Computer/Browser manifests use Swift tools 6.2 and macOS 26; Hub uses tools 6.0
and macOS 26. Computer depends on Apple containerization 0.43.0, SwiftTerm 1.20.0,
Sparkle 2.9.4 and shared native packages; Browser uses shared browser/external-tool
packages and Sparkle; Hub links Noodle's own HubCore/runtime plus HubLink. This
favours process/protocol adapters, not core library dependencies.
[Computer manifest](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Computer/Package.swift),
[Browser manifest](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Browser/Package.swift),
[Hub manifest](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Hub/Package.swift)

## Computer

**Problem solved:** persistent Linux desktop/shell environments with terminal and
file transfer, without requiring Docker Desktop. Deployment needs Apple silicon,
macOS 26+, the companion application and downloaded images. Linux guests have
separate VM boundaries; host files/clipboard are not automatically shared. Transfers
are explicit; guest network can reach the LAN. Shell networking can be disabled,
whereas Desktop requires it. The guest account has passwordless sudo: a non-root
default is not restricted guest administration.
[Computer documentation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Computer/README.md)

Local Mac is a materially different experimental backend: separate standard macOS
account, shared host hardware/network and account-level filesystem access, not a
VM. It needs separately approved login/background, screen-recording/accessibility
capabilities. Experimental Windows guests require macOS 27 and Windows licensing;
neither backend should inherit a Linux-VM isolation label. Documented agent actions
include terminals/files/previews, not general guest click/type control.
[Backend-specific documentation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Computer/README.md#local-mac-experimental)

**Integration evidence:** users can enable external agents and invoke the bundled
`noodle-computer` CLI or its `mcp` mode. Operations cover listing/starting guests,
terminal open/read/write/close, file transfers and scoped management/borrowing.
The app prompts for first caller access and each creation; callers can see their
own/lent guests and manage only what they created. Access revocation closes that
caller's terminal sessions in the service implementation.
[External client implementation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Computer/Sources/ComputerExternal/ComputerExternalClient.swift),
[External service](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Computer/Sources/NoodleComputer/ComputerExternalService.swift)

**Astrid fit:** potential local subprocess adapter for a specifically granted
computer/terminal, or MCP adapter once that capability is authorized. Astrid must
identify the guest target, prevent unauthorized host-side upload/download paths,
retain output limits and distinguish guest permissions from host permissions.
Do not expose management/delete commands merely because discovery advertises them.
One guest shared by several agents is shared mutable state, not worker isolation.

## Browser

**Problem solved:** persistent per-browser website identities/tabs and background
actions in pages the user signed into. This is a separate embedded browser, not
Safari/Chrome automation; it does not borrow their profiles/extensions. Browser
code and docs expose reading, navigation, pointer/form actions, screenshots,
history/bookmarks and explicit files, with experimental website-provided tools.
It does not require Safari automation, screen-recording or accessibility permission
for these embedded-page operations. These are website-account actions, not OS
desktop authority.
[Browser documentation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Browser/README.md)

**Integration evidence:** `noodle-browser` CLI and MCP mode expose external
operations. External agents need enablement and caller approval, with created/lent
browser grants and ownership-restricted management. Signed-in tabs confer real
account permissions; pausing prevents new actions but cannot undo committed ones.
WebMCP support is experimental and must not become trusted instructions/consent.
Some login/passkey/SSO/media flows and background pages may not behave like a
regular browser; live compatibility remains untested here.
[Browser external client](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Browser/Sources/BrowserExternal/BrowserExternalClient.swift),
[Browser protocol](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Browser/BrowserProtocol/Sources/BrowserBridge/Protocol.swift)

**Astrid fit:** complementary browser action executor behind a typed adapter,
initially one borrowed browser and a read-only or draft workflow. Astrid's
delegation should name allowed browser/account/operations; origin/account drift and
irreversible submission need explicit policy. Do not let the desktop presentation
webview double as a signed-in agent browser with Astrid IPC privileges.

## Local protocols, auth and cancellation

Computer/Browser have internal Unix-domain socket bridges, but they are not open
anonymous third-party sockets. The external connection verifies peer audit-token
code signatures against Noodle app/CLI identity and team, checks private socket
ownership and bounds frames. The signed CLI discovers its launching process's
ancestry and sends caller identity for app grants. Sandboxed ancestors and Noodle
bot processes are explicitly rejected; unsigned callers are separately identified
and warned about. A direct Rust socket client cannot be assumed accepted.
[External connection](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Shared/ExternalTools/Sources/NoodleExternalTools/ExternalConnection.swift),
[Caller ancestry](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Shared/ExternalTools/Sources/NoodleExternalTools/ExternalCallers.swift)

The MCP entry point uses newline-delimited JSON-RPC stdio, initialization plus
tools/list and tools/call, with explicit supported protocol versions. Its inspected
handler ignores notifications and provides no dedicated cancellation-notification
handler. Killing an Astrid-launched CLI/MCP client does not prove a guest command
or website action stopped. A real adapter must test operation timeout, revocation,
terminal close, late results, committed effects and reconnect; remote uncertainty
cannot be converted into a successful cancel or retried mutation.
[MCP server implementation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Shared/ExternalTools/Sources/NoodleExternalTools/MCPServer.swift)

**Recommendation:** invoke independently installed signed CLIs through a typed
Rust adapter with explicit argv and scoped paths; evaluate MCP where useful. Do
not spoof signatures, alter sandbox checks or integrate internal sockets by
assuming schema visibility means authorization. Tauri selection does not decide
release App Sandbox packaging; Noodle's external ancestry rule is a real constraint
for a future sandboxed distribution, not a blocker to today's assessment.

## Hub

**Problem solved:** always-on Mac hosting Noodle bots/harness sign-ins for paired
users/devices. Users/plans control available harnesses; one-use 15-minute invitation
flows and device revocation are documented. It is a menu-bar host application,
not a general inference endpoint or computer sandbox service.
[Hub documentation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Hub/README.md)

**Protocol evidence:** HubLink implements versioned Noodle requests/events over
QUIC, TLS 1.3 and pinned peer keys, with bounded messages. Enrollment proves
device-key ownership; instructions describe UDP reachability and optional router
port forwarding. Request schemas manage Noodle bots, conversations and assigned
resources, and HubAccess uses Noodle's HarnessProvider/plan/ownership model.
No documented contract to load/host an arbitrary Astrid Rust runtime was found.
[Transport](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Shared/HubLink/Sources/HubLink/LinkTransport.swift),
[Link protocol](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Shared/HubLink/Sources/HubLink/LinkProtocol.swift),
[Access model](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/Sources/HubCore/HubAccess.swift)

**Authority caveat:** Noodle documents copied harness sign-ins remaining readable
by bots, common account/quota/revocation, broad shared-bot access and administrator
impersonation of non-admin users. Its own chat policy automatically approves
harness tool requests within assigned access. These are Noodle contracts, not
acceptable substitutes for Astrid's explicit per-action permission boundary or
private credential custody. Do not claim this research audits their security.
[Security documentation](https://github.com/pdparchitect/noodle/blob/cb26ecf632e13ceda316d8b49ce8e0246a1dd70a/docs/security.md)

**Astrid fit:** adopt ideas for pinned pairing, revocation, separate UI/host/target
placement and incremental remote results. Do not port Hub's Swift UI/runtime or
its authority semantics. A compatible client would still delegate to Noodle's
runtime unless upstream adds an explicit Astrid host/target bridge. Rust QUIC/JSON
interoperability is technically plausible, but semantic/authorization compatibility
is unconfirmed. Recommendation: inspiration and a future bounded partnership
investigation, not a library dependency or Hub-as-model-provider adapter.

## Reuse and possible partnership disposition

| Candidate | Decision now | What a future investigation must prove |
| --- | --- | --- |
| Computer VM stack | Backend integration candidate; no new Astrid VM platform | Released CLI schema/capabilities, caller grants, guest lifecycle, cancellation and explicit transfer scope |
| Browser app | Backend integration candidate; preserve separate presentation/execution surfaces | Released action semantics, origin/account scope, revocation, uncertain committed effects and testable policy |
| External-tool schemas | Adopt typed/versioned/result concepts; avoid source coupling | Upstream compatibility commitment or pinned-release strategy; no anonymous API assumption |
| Hub remote architecture | Adopt concepts/inspiration; no current integration | Whether upstream supports external runtimes or separately authenticated action targets without surrendering Astrid's authority |
| Noodle whole app/runtime | No wholesale integration | Would conflict with Astrid-owned orchestration and learning goals |

Potential collaboration questions, not outreach: Will upstream support stable
CLI/MCP contracts and capability discovery? Can it expose explicit operation IDs,
cancellation and committed/uncertain results? Can Astrid have a named caller/target
bridge without importing copied provider credentials or Noodle automatic approval?
Can host pairing serve independently implemented Rust clients? None is a confirmed
partnership or a prerequisite to the first desktop workflow.
