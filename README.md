# LeadFinder

Local-first lead discovery and calling workstation for Maz Works.

LeadFinder is a Tauri desktop app for finding local businesses, importing lead data, reviewing evidence, and managing a structured outbound calling workflow. The frontend is React + TypeScript; the desktop/backend layer is Rust + SQLite.

## Current status

**Prototype / pre-V1. Do not use for live cold calling yet.**

The core desktop shell, local database, CSV import, Gosom integration, and local Ollama model hooks exist, but the compliance and verification gates are still being completed. See [`docs/V1_AUDIT_AND_PLAN.md`](docs/V1_AUDIT_AND_PLAN.md) for the current audit and release gates.

## Stack

- React 19 + TypeScript + Vite
- Tauri 2
- Rust + SQLite (`rusqlite`)
- Gosom for local-business discovery
- Ollama for bounded local model assistance

## Development

```bash
npm install
npm run tauri dev
```

Useful checks:

```bash
npm run lint
npm run build
cd src-tauri && cargo check
```

## Product principle

LeadFinder should fail closed: a lead must not become callable just because data exists. Verification, suppression/compliance checks, evidence, and deterministic eligibility belong below the UI.

## Repository

Built by Maz Works. Source: https://github.com/manazoid4/leadfinder
