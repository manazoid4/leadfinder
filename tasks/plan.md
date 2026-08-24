# LeadFinder V1 implementation plan

Source of truth: `docs/V1_AUDIT_AND_PLAN.md`. Execute in order. Do not begin UI polish or AI expansion before the truth, discovery, evidence, and eligibility checkpoints pass.

## Task 1 — Repair the SQLite truth layer

Scope: Rust database initialization, migrations, health, and persistence tests.

- Replace ad-hoc schema/seed SQL with versioned migrations and parameterized writes.
- Remove the callable production seed; allow explicit development fixtures only.
- Add tables for jobs, raw leads, canonical leads, stage results/evidence, screenings, suppressions, calls, callbacks, and settings.
- Return measured SQLite/migration health to the frontend.
- Test create, reopen, migrate, list, outcome, and restart persistence.

Acceptance: a fresh and existing database opens without error; no unscreened lead is eligible; Rust tests fail before and pass after the repair.

Verification: `cargo test`; launch installed/dev app twice against the same data directory; inspect measured health and persisted outcome.

## Task 2 — Correct Gosom and CSV ingestion

Depends on: Task 1.

- Define one source-independent `RawLead` contract and only two adapters: Gosom and CSV.
- Resolve/package Gosom through the actual Tauri resource/sidecar path.
- Prefer Gosom's structured JSON output internally where it is complete and stable; retain RFC 4180 CSV for user imports.
- Parse RFC 4180 CSV by normalized header names; never use positional `split(',')`.
- Map Gosom title, category, address, website, phone, Maps/source URL, and place/source ID.
- Persist rejected rows with field-level errors and job counters.
- Run a small preflight discovery before a requested full search so configuration/source failure is surfaced early without burning the whole job.

Acceptance: a real Derby search and representative quoted CSV map every field correctly; malformed rows are visible rejects; CSV still works when Gosom is missing.

Verification: parser fixtures from real Gosom headers/quoted rows; adapter tests; installed binary smoke test; raw-to-canonical spot check.

## Task 3 — Normalize, deduplicate, and persist jobs

Depends on: Task 2.

- Canonicalize UK phones, domains, names, areas, and stable source identifiers.
- Enforce uniqueness for normalized phone and suitable domain/place identifiers.
- Persist source job, requested target, stage counts, timestamps, cancellation, resume state, and per-lead errors.
- Run Gosom off the Tauri main thread with progress events and unique atomic output paths.
- Make completed stages idempotent and retry only failed/incomplete work.

Acceptance: repeated import/discovery yields zero duplicate callable phones; close/reopen resumes; one bad row/source does not fail the job.

Verification: duplicate fixtures; cancellation/resume integration test; 100-lead partial-failure scenario.

### Checkpoint A — Real discovery

Do not proceed until `roofing contractors / Derby / 100` produces correctly mapped, deduplicated, persisted businesses and CSV provides the same downstream result.

## Task 4 — Implement evidence passes 1-3

Depends on: Checkpoint A.

- Persist pass records for schema/normalization, identity/dedupe, and source/contact corroboration.
- Record status, timestamp, source, evidence payload, hash, and error per pass.
- Require corroboration rules appropriate to each available contact field; represent unknown honestly.
- Derive progress from records rather than a mutable counter.

Acceptance: the UI/backend can explain why each lead passed, failed, blocked, or remains uncertain; no manual/model-controlled `verification_count` exists.

Verification: pure rule tests and database restart tests for every state transition.

## Task 5 — Implement conservative website probing

Depends on: Task 4.

- Add bounded cheap HTTP probing, followed by Playwright only where needed.
- Capture final URL, response history/status, rendered evidence, relevant console/network failure, and screenshot state.
- Classify 401/403/429, challenge, automation rejection, ambiguous timeout, and repeated partial rendering as blocked/uncertain.
- Require repeated DNS/connection evidence for `DEAD_SITE`.
- Emit `FORM_SUSPECT` only from user-visible browser evidence; never submit a fabricated enquiry.
- Limit Playwright workers to one while Call View is active.

Acceptance: one blocked/broken website is a local lead result; absence is never inferred from blocked/uncertain evidence.

Verification: deterministic fixtures/local test servers for success, redirect, timeout, blocked, DNS/connection, challenge, missing screenshot, and form-render failure.

## Task 6 — Deterministic adjudication and opener

Depends on: Task 5.

- Make pass 5 a pure deterministic adjudicator over persisted evidence.
- Implement `GapReason + Confidence + Evidence -> opener` as a pure Rust domain function.
- Add every v6.1 opener branch and default uncertain/blocked neutral wording.
- Keep direct `NO_BOOKING_PATH` opener disabled behind the validation gate.
- Add regression tests proving AI/database/frontend cannot alter pass 5 or opener.

Acceptance: five-pass completion works with Ollama and 9router offline; `NO_WEBSITE` never claims a website visit; unsupported claims are impossible by construction.

Verification: exhaustive table-driven domain tests and mutation/bypass attempts.

### Checkpoint B — Truthful evidence

Do not proceed until five named persisted pass records explain each qualified/rejected lead and all conservative-classification/opener tests pass.

## Task 7 — TPS/CTPS and Rust eligibility

Depends on: Checkpoint B.

- Add GUI-configured provider setup and measured health; no manual environment editing.
- Persist TPS/CTPS result, provider evidence, checked time, and normalized number.
- Enforce <=28-day freshness, own suppression, 90-day redial, callback override timestamp, UK calling window, valid phone, and evidence-qualified state in Rust.
- Make provider missing/error/stale fail closed.
- Make Do Not Call suppress phone/domain duplicates immediately and atomically.
- Persist the exact eligibility snapshot on every call attempt.

Acceptance: only Rust-generated eligible leads enter the queue; all required positive/stale/missing/suppressed/redial/window cases are rejected; callback override is narrow and tested.

Verification: table-driven policy tests plus command-level bypass tests from arbitrary frontend input.

## Task 8 — Finish the call workflow and metrics

Depends on: Task 7.

- Build deterministic first-eligible queue selection and persisted queue position.
- Show identity, dialability summary, evidence timestamps, and opener immediately.
- Add outcome shortcuts 1-6, required callback date/time, Save + Next, and due-callback queue.
- Prevent duplicate cold attempts and apply redial rules.
- Persist/export funnel counts and rejection reasons.

Acceptance: Maz can complete 10 calls, callbacks appear only when due, Save + Next advances, and restart restores queue/outcomes.

Verification: end-to-end call-loop tests with duplicate, suppression, callback, restart, and no-eligible explanations.

## Task 9 — Rebuild the GUI as a Windows workbench

Depends on: backend states from Tasks 1-8. It may begin earlier only behind truthful command contracts.

- Set default 1280x800 and minimum 900x640.
- Implement responsive queue/dossier layout and sub-900 Queue/Lead tabs without overflow.
- Add visible discovery labels, target count, primary Find leads, secondary keyboard-accessible CSV, progress, Cancel/Resume, and useful empty/error states.
- Derive every status and disabled reason from backend state.
- Use system typography, normal case, restrained yellow, visible focus, 44 px controls, persistent live feedback, and accessible contrast.
- Remove terminal styling, side stripes, repeated uppercase eyebrow labels, and decorative card repetition.

Acceptance: every action works or explains its disabled state; no false readiness; no overflow at release widths/scaling; keyboard and screen-reader paths work.

Verification: Impeccable audit, automated browser/component checks, keyboard pass, Windows screenshots at 900/1024/1280/1440 and 125-200% scaling.

## Task 10 — Optional local-model advisory

Depends on: deterministic product through Task 8.

- Maz Fast plans bounded discovery queries with validated deterministic fallback.
- Maz Smart reviews evidence discrepancies asynchronously; validate <=60 words and unsupported claims.
- Cache Maz Smart by probe hash with TTL; empty/invalid/unavailable is a non-blocking warning.
- Use exact advisory label and keep the panel collapsed/secondary.

Acceptance: disabling Ollama changes neither discovery fallback, pass completion, eligibility, opener, nor calling.

Verification: offline, timeout, malformed, empty, hallucinated-number, unsupported-service, cache-hit, and cache-expiry tests.

## Task 11 — Validation, package, and installed UAT

Depends on: Tasks 1-10.

- Label at least 50 real UK trade sites across the five agreed categories.
- Keep direct `NO_BOOKING_PATH` disabled until false positives / all predictions <=5% and unsupported opener claims = 0.
- Package NSIS `LeadFinder-Setup.exe`; test fresh install and first-run health.
- Discover/import 100 real businesses, manually verify at least 50 identity/phone/domain mappings, confirm zero duplicate callable phones and zero compliance bypasses.
- Record 10 outcomes; restart; verify persistence.
- Independently simulate Gosom, CSV-row, probe, TPS, Ollama, and 9router failures.

Acceptance: every release gate in `docs/V1_AUDIT_AND_PLAN.md` has retained evidence. Only then mark V1 working and enable real cold-call use.

### Checkpoint C — Release

Ship only after the installed application, not merely development mode, passes the full UAT and regression suite.
