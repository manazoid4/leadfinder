# LeadFinder v1 implementation plan

LeadFinder is a standalone Windows/Tauri product for Maz Works client acquisition. It must not be built inside JobFilterV1 or MazOS.

## Vertical slices

1. **Desktop shell + call slice — blocked by audit findings**
   - Packaging exists, but SQLite initialization fails and the seeded lead is unsafe. Eligibility, opener, and Save + Next are not complete.
2. **Discovery — blocked by audit findings**
   - Maz Fast query planning and CSV/Gosom scaffolding exist, but the packaged Gosom path is wrong and the positional CSV parser corrupts real Gosom rows.
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

The installed shell is a prototype and must not be used for live cold calling. The multi-perspective audit found blocking SQLite, Gosom-path, CSV-mapping, verification, compliance, calling-flow, and responsive-UI failures. The authoritative repair sequence and release gates are in [V1_AUDIT_AND_PLAN.md](V1_AUDIT_AND_PLAN.md); executable tasks are in `tasks/plan.md` and `tasks/todo.md`.

## Operating constraints

- `leadfinder` is the only target repository for this product.
- JobFilterV1 is read-only reference material; MazOS and `mazos-site` remain separate.
- No secrets, customer contact, production deployment or automatic calling.
- Every future prompt batch is additive context; update this plan and the relevant design/tests before expanding scope.
