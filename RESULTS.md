# LeadFinder results log

### BUILT
- Transactional SQLite schema v2: phone optional, outcomes preserved, evidence/research/partner/model-cache tables.
- RFC 4180 ingestion, strong domain/name/phone/place-ID dedupe, nine-stage pipeline, two template views, config-driven noindex demos, fixed manual outreach copy.
- Separate web discovery for online companies; Gosom remains the premises path.
- 9router-only OpenAI-compatible client with 2KB input guard, per-run cap, token logging, one escalation, and content-hash cache.

### REUSED
- Existing Tauri/Vite/Rust/SQLite workstation, Gosom sidecar packaging, and the shared engraving renderer across both starting templates.

### BORROWED
- Konva MIT directional Sobel emboss, ported without the scene-graph dependency.
- ProjectDiscovery httpx v1.10.0 sidecar using wappalyzergo technology fingerprints.

### REAL TESTS
- `paarsawahid.com`: REJECT — current Easify fingerprint means it already has a personalisation app.
- `rfidwallets.co.uk`: QUALIFY for research — Shopify detected, no supplied reject fingerprint, live £24.99 wallet and real blank product image used.
- Live DuckDuckGo HTML web discovery returned 12 results for a UK personalised engraved wallet query.
- Published demo passed mobile browser checks: 1280px canvas, interactive engraving, copy-link, noindex, zero console errors.
- Duplicate, malformed CSV, phone-less migration, outcome preservation, raw-HTML model guard, >2KB model guard, and sequential five-pass gate are automated tests.

### DEMO LINKS
- https://leadfinder-tan-seven.vercel.app/#/demo/rfid-wallets-uk

### FAILURES FIXED
- Removed the phone NOT NULL blocker without dropping legacy outcomes.
- Removed unsafe seed, positional CSV splitting, mutable eligibility counter, Ollama route/fallback, stale model docs, false-ready UI, and desktop-width leakage into mobile demos.
- Dead URLs and sidecar/model failures fail loudly and leave leads unqualified; image failures render an explicit unavailable state.
- Missing owner name uses a neutral greeting and never invents a person.

### NEXT
- Maz manually sends the approved DM after confirming the public DM path and the remaining evidence passes; automated sending stays prohibited.
- Add another real config for Template 2 from the Gosom campaign and repeat without component code.
