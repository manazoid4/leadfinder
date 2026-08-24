# Local model + automatic lead discovery research

Status: research only; implementation awaits Maz's confirmation.

## Local machine findings

- Ollama is running at `http://127.0.0.1:11434`.
- `/api/tags` currently exposes local models including Qwen, LFM, Gemma, Granite, Nemotron and Llama variants. No explicit `Maz Fast` or `Maz Slow` model alias was discoverable in the local Ollama model list, so the app should use configurable role mappings rather than hard-code guessed names.
- The Ollama chat API supports structured JSON output, non-streaming responses, runtime options and keep-alive controls. These are suitable for query planning, evidence adjudication and bounded lead scoring, not for inventing business facts.

## Primary-source findings

- Ollama's official chat API documents `POST /api/chat`, required `model` and `messages`, optional JSON `format`, `stream`, `options` and `keep_alive`: https://docs.ollama.com/api/chat
- Ollama's official tags API documents `GET /api/tags` for enumerating installed models: https://docs.ollama.com/api/tags
- Gosom's official scraper supports CSV/JSON output, configurable concurrency, fast mode, email extraction and a REST job API. Its README warns that higher concurrency can increase blocking/failures and that fast mode is beta: https://github.com/gosom/google-maps-scraper

## Proposed product boundary

Local models should orchestrate and adjudicate the deterministic pipeline. They must not be the source of truth for a phone number, website, service, absence claim, TPS status or opener. Every displayed fact must retain source evidence and timestamp.

## Five verification passes

1. Input/schema validation and canonical normalization.
2. Identity dedupe using normalized business name, phone, domain and address.
3. Source corroboration: scraper record versus fetched business website/search evidence.
4. Conservative website probe using HTTP then browser evidence; blocked/ambiguous results remain uncertain.
5. Independent adjudication: deterministic checks first, then a local model review restricted to captured evidence; disagreements are flagged for review rather than silently promoted.

Five passes means five evidence checks, not five blind repeated requests. Re-fetch only when evidence conflicts or is stale, to reduce blocking and preserve source safety.
