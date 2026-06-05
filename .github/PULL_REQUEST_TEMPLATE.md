## Summary

- 

## Type

- [ ] `fix:` bug fix, patch release
- [ ] `feat:` new feature, minor release
- [ ] `docs:` documentation only
- [ ] `test:` tests only
- [ ] `chore:` / `ci:` maintenance, no release
- [ ] Breaking change: PR title uses `!` or description includes
      `BREAKING CHANGE:`

## Release Please

- [ ] PR title follows Conventional Commit style, for example
      `fix: handle missing valves block`.
- [ ] I did not manually bump versions unless this is release/version
      maintenance.
- [ ] If version metadata changed, I kept `Cargo.toml`, `Cargo.lock`,
      `.release-please-manifest.json`, `CHANGELOG.md`, and the VS Code
      extension (`editors/vscode/package.json`, `package-lock.json`) in sync.

## Tests

- [ ] Ran `cargo fmt -- --check` and `cargo clippy --locked -- -D warnings`.
- [ ] Ran `cargo test --locked` (and `make test-scripts` if scripts changed).
- [ ] Ran `make docs-check` if code or docs changed.
- [ ] Added manual verification notes below if automated coverage is not
      practical.

## Verification

```text

```

## Notes

-
