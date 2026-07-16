# Auto-update — activation guide

The desktop app's auto-updater is **fully scaffolded but inert**. The plugin, the
`check_update` / `install_update` commands, the in-app "Update & restart" banner, and the
CI signing hooks are all in place. Until you complete the steps below, `check_update`
returns an error and the banner simply never appears — so the app and the release pipeline
work exactly as they do today.

Activating it takes three things: a signing keypair, two GitHub secrets, and a config block.

## How it works

Tauri's updater checks a JSON feed (`latest.json`) published on each GitHub Release. If it
lists a version newer than the running app **and** the bundle's signature verifies against
the public key baked into the app, the banner offers to download, install, and restart.
The private key never leaves your secrets; a tampered update fails verification and is
rejected.

## 1. Generate a signing keypair (once)

```sh
npm run tauri signer generate -- -w ~/.tauri/trackly-updater.key
```

This prints a **public key** and writes the password-protected **private key** to the file.
Keep the private key and password secret — anyone with them can sign updates your users
will trust.

## 2. Add two GitHub Actions secrets

In the repo: **Settings → Secrets and variables → Actions → New repository secret**

| Secret | Value |
|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | the entire contents of `~/.tauri/trackly-updater.key` |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the password you chose in step 1 |

`.github/workflows/desktop-release.yml` already reads both — no workflow edit needed.

## 3. Turn on the updater in `src-tauri/tauri.conf.json`

Add `createUpdaterArtifacts` to the `bundle` block, and add a `plugins.updater` block with
your **public key** from step 1:

```jsonc
{
  "bundle": {
    "createUpdaterArtifacts": true
    // ...existing bundle settings
  },
  "plugins": {
    "updater": {
      "endpoints": [
        "https://github.com/vehutech/trackly/releases/latest/download/latest.json"
      ],
      "pubkey": "PASTE_YOUR_PUBLIC_KEY_HERE"
    }
  }
}
```

> The `pubkey` is required and must be the real key — a placeholder makes `tauri build`
> fail. That's exactly why this block is documented here rather than committed inert.

## 4. Cut a release

```sh
git tag vX.Y.Z && git push origin vX.Y.Z
```

`tauri-action` now signs each bundle, generates `latest.json`, and uploads it to the
release. Apps on an older version will show the update banner on next launch.

## Notes

- **Installer signing is separate.** The updater signature (above) proves an update is
  authentic; it does **not** stop macOS Gatekeeper / Windows SmartScreen warnings on first
  install. Removing those needs an **Apple Developer ID** and a **Windows code-signing
  certificate**, wired via their own secrets in the same workflow `env`.
- **JS-side updater (optional).** The current flow calls the plugin from Rust, so no extra
  capability is needed. If you ever call `@tauri-apps/plugin-updater` from the frontend
  instead, add `"updater:default"` to `src-tauri/capabilities/default.json`.
- **Testing.** Build a `vX.Y.Z-1` locally, install it, then release `vX.Y.Z` — the older
  build should offer the update.
