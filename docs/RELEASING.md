# Releasing a new version

Sortty ships signed Windows installers built by CI and distributed as GitHub Releases, with in-app auto-update checking against the latest release. See [ADR 0012](adr/0012-manual-update-checks.md) for why update checks are user-triggered rather than automatic.

## Cutting a release

1. Bump the version in all three places (they must match):
   - `package.json`
   - `src-tauri/Cargo.toml`
   - `src-tauri/tauri.conf.json`
2. Commit: `chore: bump version to X.Y.Z`
3. Tag and push: `git tag vX.Y.Z && git push origin vX.Y.Z`
4. [`.github/workflows/release.yml`](../.github/workflows/release.yml) builds signed installers and a `latest.json` update manifest, then drafts a GitHub Release. Review the draft and publish it — nothing is public until you do.

There's no automated version-bump tooling; the three files are edited by hand and kept in sync by convention.

## One-time setup: updater signing key

The updater plugin verifies every downloaded update against a public key baked into the app (`src-tauri/tauri.conf.json`'s `plugins.updater.pubkey`). This only needs to be done once, ever, per app identity:

1. Generate a keypair locally (never in CI): `npx tauri signer generate -w ~/.tauri/sortty.key`.
2. Add the private key (and password, if you set one) as GitHub repository secrets: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (Settings → Secrets and variables → Actions).
3. Paste the generated **public** key into `tauri.conf.json`'s `plugins.updater.pubkey` (safe to commit — it's the placeholder `REPLACE_WITH_GENERATED_PUBLIC_KEY` until this step is done).
4. Push a real tag to validate the whole pipeline end-to-end before relying on it.

Keep the private key and its backup somewhere durable outside GitHub — losing it means every future release needs a new keypair, and existing installs can no longer verify (and therefore won't accept) updates signed with the new one.
