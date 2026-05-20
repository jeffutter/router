### Pick up hickory 0.26.1 to close two upstream DNS DoS advisories ([RUSTSEC-2026-0119](https://rustsec.org/advisories/RUSTSEC-2026-0119), [RUSTSEC-2026-0120](https://rustsec.org/advisories/RUSTSEC-2026-0120))

The router's DNS resolver (via `hickory-resolver`) inherits two upstream advisories in `hickory-proto` / `hickory-net` 0.26.0.  Both are fixed in 0.26.1, which is now pinned in `Cargo.lock`.

Source-built consumers were already insulated by caret-semver dependency declarations; this change picks up the fix in Apollo's pre-built binaries and Docker images.

By [@carodewig](https://github.com/carodewig) in https://github.com/apollographql/router/pull/9321
