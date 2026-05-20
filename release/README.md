# Waey Release Setup

Waey uses Tauri's updater plugin. The plugin is installed, but updater signing is not enabled
in `tauri.conf.json` until we generate the production signing key and choose the final release
endpoint.

## Build Commands

- `npm run build` builds the frontend.
- `npm run app:build` builds desktop bundles through Tauri.

## Enable Updater For Release

1. Generate an updater key after Rust is installed:

   ```powershell
   npm run tauri signer generate -- -w ~/.tauri/waey.key
   ```

2. Add this to `src-tauri/tauri.conf.json`:

   ```json
   {
     "bundle": {
       "createUpdaterArtifacts": true
     },
     "plugins": {
       "updater": {
         "pubkey": "PUBLIC_KEY_CONTENT",
         "endpoints": [
           "https://github.com/waey/waey/releases/latest/download/latest.json"
         ],
         "windows": {
           "installMode": "passive"
         }
       }
     }
   }
   ```

3. Build release artifacts with the private key in the environment:

   ```powershell
   $env:TAURI_SIGNING_PRIVATE_KEY="Path or content of your private key"
   $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
   npm run app:build
   ```

4. Publish a `latest.json` shaped like `release/updater.example.json`.
