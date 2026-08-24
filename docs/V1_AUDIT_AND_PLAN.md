# LeadFinder V1 audit and delivery plan

Date: 2026-08-24
Status: audit complete; implementation not started in this pass

## Executive verdict

LeadFinder is currently a packaged prototype, not a working V1. The Tauri shell, Rust-owned SQLite direction, Gosom binary, installer, and local Ollama models exist, but the installed application cannot yet produce a trustworthy, legally callable lead queue.

The shortest route to a useful Maz Works product is:

1. repair SQLite and remove the fail-open test lead;
2. make one real Gosom search import correctly through the same contract as CSV;
3. persist five distinct deterministic evidence passes;
4. enforce TPS, CTPS, suppression, freshness, redial, callback, and calling-window rules in Rust;
5. finish the call desk with evidence, truthful opener, callback handling, and Save + Next;
6. pass the 50-site detector gate and a fresh-install Windows UAT.

Do not spend the next implementation session adding providers, CRM features, more AI, or decorative UI.

## Audit method

Two independent product/technical audits, a separate GUI audit, and an independent judge were commissioned. The repository was also examined with the locally installed `code-review-graph`, normal build and lint tooling, Rust tests, dependency audit, installed-app runtime checks, Gosom, Ollama, Impeccable, `aislop`, and competitor research.

The judge returned a **NO-GO for V1** and found no substantive disagreement between the auditors. Passing builds, packaging, and static-quality checks demonstrate tooling health; they do not demonstrate product correctness.

`code-review-graph` found only 35 code nodes in two main communities and seven shallow flows. The product is concentrated in `src-tauri/src/lib.rs` and `src/App.tsx`; no test flow covers discovery. Tauri macro and IPC edges are not fully represented by that graph, so installed runtime checks were treated as the stronger evidence.

## What actually works

- LeadFinder is a standalone repository and does not depend on JobFilterV1 at runtime.
- The intended packaged architecture is Tauri -> Rust -> SQLite -> commands -> static frontend.
- TypeScript build, Rust compilation, Clippy, and package generation pass.
- The installed Gosom binary exists and reports version `v1.17.4-beca11f`.
- Ollama exposes Maz Fast (`phi4-mini:latest`) and Maz Smart (`lfm2.5-8b:latest`).
- Maz Fast produced useful Derby roofing search variations in a direct runtime test.
- Production dependency audit reported no known vulnerabilities.

These are useful foundations, not proof of the lead-acquisition workflow.

## P0 failures

### 1. SQLite truth layer is broken

The seed SQL escapes an apostrophe using a backslash. SQLite rejects it with `near "s": syntax error`. Since connection setup retries that insert, database-backed commands can fail repeatedly. The installed database existed but contained no leads during the audit.

Fix with versioned migrations, parameterized writes, no production callable seed, and Rust tests for create, reopen, migrate, list, outcome, and restart persistence.

### 2. Automatic discovery cannot currently run

The Rust code looks for `resource_dir/gosom.exe`; the installer places the binary under `resources/gosom.exe`. Even after correcting this lookup, ingestion is unsafe.

The importer assumes five positional fields and calls `split(',')`. Gosom output uses a larger named schema beginning with fields such as input ID, Maps link, title, category, and address. The current implementation would shift data into the wrong columns and break quoted values containing commas.

Fix with an RFC 4180 CSV library, header-name mapping, source-specific adapters, raw-row rejects, and a shared `RawLead -> normalize -> dedupe` pipeline.

### 3. The five-pass gate is only a counter

Imports receive `verification_count = 1`; no command advances it and no pass evidence exists. A genuine 5/5 must be derived from five persisted records:

1. schema and normalization;
2. identity and deduplication;
3. source/contact corroboration;
4. conservative HTTP/browser probe with evidence and screenshot state;
5. deterministic adjudication.

Each record needs `PASS | FAIL | BLOCKED | UNCERTAIN`, timestamp, source/evidence, hash, and error. Maz Smart may review disagreements afterward, but local AI availability or output must never be required for pass 5.

### 4. Eligibility fails open

Eligibility is stored as a trusted boolean. There is no implemented TPS, CTPS, 28-day freshness, own suppression, 90-day redial, callback override, calling window, or evidence-qualified guard. A seeded 1/5 lead is marked eligible while the UI says TPS needs a key.

Only Rust-calculated eligibility may enter the queue. Missing, errored, positive, or stale screening must fail closed. `Do not call` must immediately suppress normalized phone/domain duplicates, not merely save an outcome label.

Before live use, Maz Works also needs a documented lawful-basis, transparency, retention, objection, caller identity, and calling-line-identification decision. Official ICO guidance requires B2B live-call screening against TPS/CTPS and the caller's own suppression list: [ICO B2B marketing guidance](https://ico.org.uk/for-organisations/direct-marketing-and-privacy-and-electronic-communications/business-to-business-marketing/) and [ICO live-call compliance guidance](https://ico.org.uk/for-organisations/direct-marketing-and-privacy-and-electronic-communications/guidance-on-direct-marketing-using-live-calls/how-do-we-comply-with-the-rules-on-live-marketing-calls/).

### 5. Truthful opener and probe rules are absent

The opener is stored text, not a pure `GapReason + Confidence + Evidence -> opener` function. There is no conservative `PROBE_BLOCKED` handling, browser-evidenced `FORM_SUSPECT`, or 50-site validation gate.

Direct absence claims remain disabled until the labelled set contains at least 50 real UK trade sites and false `NO_BOOKING_PATH` predictions divided by all `NO_BOOKING_PATH` predictions is at most 5%. Unsupported opener claims must be zero.

### 6. The calling loop is incomplete

`START CALLING` scrolls rather than selecting the next eligible lead. Saving an outcome does not advance. Callbacks have no due timestamp, and the callback count is historical rather than due. There is no persisted job/queue resume or duplicate-call protection.

The V1 call loop is: select first eligible -> show evidence and deterministic opener immediately -> record outcome/callback -> Save + Next -> persist queue position -> survive restart.

## Local model boundary

### Maz Fast

Use Maz Fast only to propose bounded search-query variations. Validate and deduplicate the output. If Ollama is unavailable or output is invalid, use deterministic `{trade} in {area}` discovery. Query planning must not block CSV import or calling.

### Maz Smart

Use Maz Smart only for an asynchronous evidence-discrepancy brief. It may not:

- complete a verification pass;
- change eligibility or the opener;
- invent or infer facts absent from evidence;
- make Call View wait.

Validate at most 60 words, reject unsupported numbers, currency, percentages, services, revenue/loss claims, or other invented facts, and cache by probe hash with a TTL. Empty output is failure, not success. Label it exactly `AI BRIEF — ADVISORY, DO NOT READ ALOUD`.

## Competitor patterns worth adapting

| Project | Adapt | Improve for LeadFinder | Explicitly reject for V1 |
|---|---|---|---|
| [Dukotah/leadgen](https://github.com/Dukotah/leadgen) | Source-independent collect -> dedupe -> enrich -> suppress -> score pipeline; stage counters; export | Keep only Gosom and CSV, make every stage restartable, retain raw evidence, and put legal eligibility below the UI | Ten-source breadth and unvalidated source quantity |
| [NezbiT/pitch-doctor](https://github.com/NezbiT/pitch-doctor) | Honest `could not verify` results; evidence-backed business audit | Convert evidence into a truthful call dossier and later a Maz Works leave-behind report | Treating unknown as a sales claim |
| [noemit/tardigrade](https://github.com/noemit/tardigrade) | Browser evidence, screenshots, DOM/network/console artifacts, replayable results | Use a bounded deterministic probe rubric and retain just the evidence needed for review | Generic autonomous browser agents |
| [cuongquachc88/open-seo-checker](https://github.com/cuongquachc88/open-seo-checker) | Persisted crawl runs, progress, issues, stop/resume, per-URL findings | Make jobs first-class in SQLite and allow per-lead retry without rediscovery | Broad SEO-suite scope |
| [Houseofmvps/opentechalyzer](https://github.com/Houseofmvps/opentechalyzer) | Confidence plus verbose evidence; per-URL failure isolation | Use conservative evidence sufficiency and explicit blocked/uncertain states | Technology-detection breadth unrelated to acquisition |
| [Twenty](https://github.com/twentyhq/twenty) | Familiar dense list/detail workbench, filtering, activity history | Apply only its task-oriented interaction patterns to a local single-user call desk | CRM, cloud, multi-user, and platform architecture |
| [changedetection.io](https://github.com/dgtlmoon/changedetection.io) | Timestamped evidence, retry history, content hashes | Retain an auditable per-lead evidence history and probe hash | Server/monitoring product scope |
| [BurntSushi/rust-csv](https://github.com/BurntSushi/rust-csv) | Mature RFC-compliant Rust CSV parsing | Header aliases, source preview, row-level errors, and typed adapter boundary | Hand-rolled parsing |

The useful common pattern is not “more scraping.” It is a source-agnostic normalized record, visible stage progress, per-item failures, persisted evidence, conservative unknown states, and exportable results.

## GUI direction

The GUI audit scored the current interface 6/20. The default Tauri window is 800 px while CSS forces a 980 px minimum, causing shipped-window clipping. Several controls are inert or silently do nothing. Readiness indicators are hard-coded and contradictory.

Rebuild it as a practical daylight Windows calling workbench:

- default `1280 x 800`, minimum `900 x 640`;
- at 1100 px and wider: 320 px queue plus flexible call dossier;
- 900-1099 px: 280 px queue; below 900 px: Queue/Lead tabs, never horizontal overflow;
- visible labels for trade, area, and target count; one primary `Find leads`, secondary `Import CSV`;
- persisted stage row: Discovered -> Normalised -> Probed -> TPS screened -> Ready, with counts and Cancel/Resume;
- filters: Ready, Verifying, Callbacks, Blocked;
- call dossier order: identity/dialability -> `SAY THIS` -> evidence/timestamps -> collapsible advisory AI -> outcomes -> `Save + Next`;
- Segoe UI/system sans, normal case, 14 px base, restrained yellow for primary action/selection/attention;
- remove tiny tracked all-caps labels, thick side stripes, generic dark-terminal styling, and decorative card repetition;
- 44 px targets, visible focus, persistent `aria-live` feedback, keyboard-accessible CSV, and visible disabled reasons.

The UI may only display backend-measured `ready | checking | warning | offline` states. A 1/5, missing-TPS, positive-TPS, or stale-TPS lead can never render as Ready.

## Definition of working V1

V1 is complete only when Maz can install and run this path without a terminal:

`Find roofing contractors in Derby -> receive correctly mapped real businesses -> normalize/dedupe -> persist five evidence passes -> screen TPS/CTPS and local guards -> Start Calling -> see evidence and truthful opener -> save outcome/callback -> Save + Next -> restart without losing state.`

If Gosom fails, CSV must still complete every downstream step. If Ollama or 9router fails, the deterministic product must continue. If TPS fails, there are zero callable cold leads.

## Release gates

- Real discovery imports business name, category, address, phone, website, place/source identifiers correctly by header name.
- Reimport/research produces zero duplicate callable normalized phone numbers.
- One provider/site failure is local; job completes with accurate success/blocked/failed counts.
- Close at 67/100 and reopen resumes without rediscovering completed stages.
- Every required verification pass has persisted evidence; 5/5 is derived, never assigned.
- `PROBE_BLOCKED`, timeout, 401/403/429, or challenge never produces an absence claim.
- `FORM_SUSPECT` requires a visible browser failure; no fabricated form submission occurs.
- Rust eligibility rejects TPS-positive, CTPS-positive, stale, unscreened, suppressed, invalid, out-of-window, or premature-redial leads.
- Duplicate phone never creates repeated cold calls; callback override requires an explicit due timestamp.
- `NO_WEBSITE` never says “I was on your website”; AI cannot alter eligibility or opener.
- Maz Smart output is optional, asynchronous, evidence-bound, validated, and cached.
- No horizontal scroll at 900, 1024, 1280, or 1440 px and at 125-200% text scaling.
- All visible actions work or are disabled with an explanation.
- Fifty-site validation meets the <=5% `NO_BOOKING_PATH` false-positive gate with zero unsupported opener claims.
- Fresh installed-app UAT imports/finds 100 real UK trade leads, manually checks at least 50 identity/phone/domain mappings, records 10 outcomes, verifies restart persistence, and independently simulates Gosom, Ollama, probe, and TPS failure.
- Funnel export reports discovered -> deduped -> verified -> legally callable -> attempted -> connected -> interested -> demo booked -> won, plus rejection/failure reasons.

The executable should not be used for live cold calling until these gates pass.
