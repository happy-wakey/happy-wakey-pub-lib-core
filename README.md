# happy-wakey-pub-lib-core

Public, client-safe Happy Wakey briefing policy and SDK primitives. It is the
small, deterministic core shared by Rust desktop, Flutter FFI, WebAssembly, and
external SDK consumers. It never receives connector credentials, raw private
messages, database connections, or internal administration capabilities.

## Capabilities

- authorizes only HTTPS deep links to a provider-specific message or thread;
- requires a matching, unexpired `useful` decision at or above the `0.8`
  policy threshold and rejects all generic-feed fallbacks;
- strips an unsafe action while retaining its informational briefing card;
- ranks and caps the HUD at 64 cards so the briefing cannot become a feed;
- validates embedding metadata at 1–4,100 dimensions without accepting the
  vector or source content itself; and
- validates statistical findings as correlations only, with bounded
  coefficients, p-values, confidence intervals, and minimum sample sizes.

Wire types come from `happy-wakey-interfaces` at an immutable Git revision.

## Verify

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
```
