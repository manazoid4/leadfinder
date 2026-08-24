# LeadFinder V1 execution checklist

## P0 — next implementation session

- [ ] Repair SQLite migration/init failure and remove callable seed.
- [ ] Add Rust persistence/migration tests.
- [ ] Expose truthful backend health.
- [ ] Resolve the packaged Gosom resource path.
- [ ] Replace positional CSV splitting with RFC 4180 header mapping.
- [ ] Prove one real Derby discovery maps raw Gosom fields correctly.
- [ ] Normalize UK phone/domain/place identifiers and reject duplicates.
- [ ] Persist search jobs, progress, cancellation, per-row errors, and resume.

## Core safety pipeline

- [ ] Persist evidence passes 1-3.
- [ ] Implement conservative HTTP/browser pass 4.
- [ ] Implement deterministic adjudication pass 5.
- [ ] Implement pure evidence-bound opener engine and safety tests.
- [ ] Add TPS/CTPS provider setup and fail-closed Rust eligibility.
- [ ] Add own suppression, freshness, redial, callback, and calling-window guards.
- [ ] Complete eligible queue, due callbacks, outcomes, and Save + Next.

## Product finish

- [ ] Rebuild responsive accessible Windows workbench UI.
- [ ] Add optional validated/cached Maz Smart advisory.
- [ ] Run 50-site detector validation.
- [ ] Run fresh-install 100-lead/10-call UAT and failure simulations.
- [ ] Export funnel/rejection metrics.

## Stop conditions

- Do not use the current executable for live cold calling.
- Do not display Ready unless every Rust eligibility guard passes.
- Do not make website-absence claims from blocked or uncertain probes.
- Do not make Maz Fast, Maz Smart, 9router, or any LLM part of verification, eligibility, or opener selection.
- Do not add extra lead providers, CRM, cloud sync, softphone, autonomous agents, or decorative UI before V1 gates pass.
