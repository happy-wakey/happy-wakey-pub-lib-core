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

## Repository-local Git worktrees

- Create or use a Git worktree only when the human operator explicitly authorizes it for the current task. Concurrency or a dirty checkout is not permission by itself.
- Put every authorized worktree at `<repository-root>/tmp/worktrees/<name>`; from the repository root, use `./tmp/worktrees/<name>`. Never place worktrees beside repositories or organization directories.
- Keep `tmp`, `temp`, `tmp/worktrees`, and `temp/worktrees` ignored in the repository-root `.gitignore`. Do not commit files from those directories.
- Relocate or remove a worktree only when the operator explicitly requests it. Before removal, preserve and publish intended changes, verify its commit is represented on the target branch, and confirm there are no tracked, untracked, ignored-sensitive, or in-use files that must survive. Remove it with `git worktree remove <path>` without `--force`; never delete a worktree directory with `rm`.
