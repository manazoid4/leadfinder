# LeadFinder v1 implementation plan

LeadFinder is a standalone Windows/Tauri product for Maz Works client acquisition. It must not be built inside JobFilterV1 or MazOS.

## Vertical slices

1. **Desktop shell + call slice — in progress**
   - Rust-owned SQLite, seeded lead, deterministic eligibility/opener, Call View, outcome persistence.
2. **Discovery — planned**
   - Gosom Tauri sidecar plus CSV import fallback, normalisation and dedupe.
3. **Probe — planned**
   - Conservative HTTP/Playwright evidence, `PROBE_BLOCKED`, `FORM_SUSPECT`, gap reason and confidence.
4. **Eligibility — planned**
   - TPS/CTPS freshness, suppression, redial and calling-window guards.
5. **Validation — planned**
   - At least 50 labelled UK trade websites; direct opener remains disabled above the 5% false-positive threshold.
6. **9router advisory — planned**
   - Optional, validated AI brief beneath the deterministic opener; never required for calling.
7. **Packaging + UAT — planned**
   - `LeadFinder-Setup.exe`, first-run checks, restart persistence and installed-app acceptance.

## Current status

The initial shell is implemented locally: the frontend calls Tauri commands, SQLite is owned by Rust, a test lead is seeded, deterministic eligibility and opener text are visible, and outcomes persist by lead ID. Discovery, probe, TPS/CTPS and 9router are intentionally not claimed as shipped yet.

## Operating constraints

- `leadfinder` is the only target repository for this product.
- JobFilterV1 is read-only reference material; MazOS and `mazos-site` remain separate.
- No secrets, customer contact, production deployment or automatic calling.
- Every future prompt batch is additive context; update this plan and the relevant design/tests before expanding scope.
