import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

type Lead = {
  id: number; businessName: string; trade: string; area: string; phone: string;
  website?: string; gapReason: string; confidence: string; eligible: boolean;
  opener: string; outcome?: string; verificationCount: number;
};
type ModelStatus = { fastModel: string; smartModel: string; ollamaReady: boolean; message: string };
const outcomes = ['No answer', 'Interested', 'Callback', 'Booked', 'Not interested', 'Do not call'];

export default function App() {
  const [leads, setLeads] = useState<Lead[]>([]);
  const [selected, setSelected] = useState<Lead | null>(null);
  const [message, setMessage] = useState('');
  const [trade, setTrade] = useState('roofing contractors');
  const [area, setArea] = useState('Derby');
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [smartReview, setSmartReview] = useState('');

  async function load() {
    const next = await invoke<Lead[]>('list_leads');
    setLeads(next); setSelected(current => next.find(lead => lead.id === current?.id) ?? next[0] ?? null);
  }
  useEffect(() => { void load(); void invoke<ModelStatus>('model_status').then(setModelStatus).catch(() => setModelStatus(null)); }, []);

  async function autoFind() {
    setBusy(true); setMessage('Maz Fast is planning evidence-bound search queries…');
    try { const queries = await invoke<string[]>('plan_search', { trade, area }); setMessage(`Maz Fast created ${queries.length} queries. Gosom is discovering businesses…`); const count = await invoke<number>('discover_leads', { queries }); setMessage(`Discovered ${count} businesses. Five verification passes are required before calling.`); await load(); }
    catch (error) { setMessage(`Automatic discovery unavailable: ${String(error)}. CSV import remains available.`); }
    finally { setBusy(false); }
  }
  async function importCsv(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]; if (!file) return; setBusy(true);
    try { const count = await invoke<number>('import_csv', { contents: await file.text() }); setMessage(`Imported ${count} leads. They remain uncallable until verification reaches 5/5.`); await load(); }
    catch (error) { setMessage(`CSV import failed: ${String(error)}`); }
    finally { setBusy(false); event.target.value = ''; }
  }
  async function saveOutcome(outcome: string) {
    if (!selected) return; await invoke('save_outcome', { id: selected.id, outcome }); setMessage(`Saved: ${outcome}`); await load();
  }
  async function runSmartReview() {
    if (!selected) return; setSmartReview('Maz Smart is reviewing captured evidence…');
    try { setSmartReview(await invoke<string>('smart_review', { businessName: selected.businessName, website: selected.website ?? null, evidence: `${selected.gapReason}; confidence=${selected.confidence}; verification=${selected.verificationCount}/5` })); }
    catch (error) { setSmartReview(`Smart review unavailable: ${String(error)}`); }
  }

  return <main className="app-shell">
    <header className="topbar"><div><p className="eyebrow">MAZ WORKS · LOCAL WINDOWS WORKSTATION</p><h1>LEADFINDER</h1><p className="subtitle">Find leads. Check evidence. Make honest calls.</p></div><div className="system-strip"><span>SQLite <b>● ready</b></span><span>Discovery <b>● automatic</b></span><span>Models <b className={modelStatus?.ollamaReady ? 'ready' : 'blocked'}>{modelStatus?.ollamaReady ? '● Maz Fast + Smart' : '○ checking'}</b></span><span>TPS <b>● key needed</b></span></div></header>
    <section className="actions" aria-label="Lead actions"><input aria-label="Trade" value={trade} onChange={event => setTrade(event.target.value)} /><input aria-label="Area" value={area} onChange={event => setArea(event.target.value)} /><button className="primary" disabled={busy} onClick={() => void autoFind()}>{busy ? 'BUILDING…' : 'AUTO-FIND LEADS'}</button><label className="file-button">IMPORT CSV<input type="file" accept=".csv,text/csv" onChange={event => void importCsv(event)} /></label><button onClick={() => selected && document.getElementById('call-view')?.scrollIntoView()}>START CALLING</button><button>CALLBACKS DUE <span className="count">{leads.filter(lead => lead.outcome === 'Callback').length}</span></button></section>
    <div className="workspace"><aside className="lead-list"><div className="list-head"><div><p className="eyebrow">QUALIFICATION QUEUE</p><h2>{leads.length} lead{leads.length === 1 ? '' : 's'}</h2></div><span className="stage">5-PASS GATE</span></div>{leads.map(lead => <button className={`lead-row ${selected?.id === lead.id ? 'active' : ''}`} key={lead.id} onClick={() => setSelected(lead)}><span><strong>{lead.businessName}</strong><small>{lead.trade} · {lead.area}</small></span><span className={lead.eligible ? 'ready' : 'blocked'}>{lead.eligible ? 'READY' : `${lead.verificationCount}/5`}</span></button>)}</aside>
      {selected && <section className="call-view" id="call-view"><div className="call-header"><div><p className="eyebrow">CALL VIEW · {selected.trade.toUpperCase()}</p><h2>{selected.businessName}</h2><p className="muted">{selected.area} · {selected.phone}</p></div><span className={selected.eligible ? 'status ready' : 'status blocked'}>{selected.eligible ? 'ELIGIBLE' : 'VERIFYING'}</span></div><div className="evidence"><div><span className="label">GAP REASON</span><strong>{selected.gapReason}</strong></div><div><span className="label">CONFIDENCE</span><strong>{selected.confidence}</strong></div><div><span className="label">WEBSITE</span><strong>{selected.website ?? 'Not found'}</strong></div><div><span className="label">VERIFICATION</span><strong>{selected.verificationCount}/5</strong></div></div><article className="say-this"><p className="eyebrow">SAY THIS · DETERMINISTIC OPENER</p><p>{selected.opener}</p></article><article className="ai-brief"><p className="eyebrow">MAZ SMART · ADVISORY REVIEW</p><button onClick={() => void runSmartReview()}>RUN SMART REVIEW</button><p>{smartReview || 'Restricted to captured evidence; never changes eligibility or the opener.'}</p></article><div className="outcomes"><p className="eyebrow">OUTCOME</p>{outcomes.map(outcome => <button key={outcome} disabled={!selected.eligible} onClick={() => void saveOutcome(outcome)}>{outcome}</button>)}</div>{message && <p className="saved" role="status">{message}</p>}</section>}
    </div>
  </main>;
}
