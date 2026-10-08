Investigate MadeAgents/mobile-use (https://github.com/MadeAgents/mobile-use) as a potential future Android GUI-execution backend for Astrid.
Do not integrate it yet.
Study its current architecture and document:

- observation representation,
- screen/UI parsing,
- action representation,
- Android/ADB execution boundary,
- planning loop,
- reflection loop,
- memory/progress representation,
- model interface,
- task termination,
- cancellation behavior,
- permission/safety assumptions,
- AndroidWorld/AndroidLab evaluation approach,
- Python/runtime dependencies,
- MIT-license obligations,
- and which pieces are architecture-independent.
  Then map those concepts onto Astrid's existing runtime abstractions.
  Categorize each useful idea as:
  adopt concept / adapt implementation / backend integration / inspiration only / reject.
  Specifically evaluate whether a future GuiExecutor or device-capability interface could allow MobileUse to run as an external Python process/service while Astrid remains Rust-native.
  Do not create abstractions in the production code merely to accommodate this future integration.
  Produce research documentation only unless an immediate defect in Astrid's current architecture would make such an adapter fundamentally impossible.
