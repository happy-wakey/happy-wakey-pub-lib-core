# Public library boundary

This repository is public and client-safe. It may contain deterministic
briefing policy, validation, ranking, and cross-client helpers. It must not
contain connector credentials, raw private-message content, database access,
internal admin capabilities, model-provider secrets, or network clients.

`happy-wakey-interfaces` is the wire-type authority and must be pinned to an
immutable Git revision. A social deep link is authorized only when its
decision is useful, above policy threshold, unexpired, internally consistent,
HTTPS, and points to a provider-specific item rather than a generic feed.

Every behavior change requires unit tests. Run formatting, Clippy with warnings
denied, the full test suite, and the repository safety scan before publication.
