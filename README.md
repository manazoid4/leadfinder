# LeadFinder

Local-first lead discovery, research, demo generation, manual outreach, and pipeline tracking for Maz Works.

## Run

```powershell
npm install
npm run tauri dev
```

The desktop app stores leads in SQLite under the Tauri app-data directory. It bundles Gosom for premises-based Google Maps discovery and ProjectDiscovery httpx/wappalyzergo for deterministic technology detection. Online companies use the separate web-search path.

Model calls go only through 9router's OpenAI-compatible `/v1/chat/completions` endpoint. Inputs are capped at 2KB, raw HTML is rejected, calls are capped per process run, and there is no local-model fallback.

## Verification

```powershell
npm run lint
npm run build
cd src-tauri
cargo test
cargo clippy -- -D warnings
```

Demo routes are config-driven under `public/demo-configs`, use `noindex`, and render with the shared engraving module. Example: `#/demo/rfid-wallets-uk`.
