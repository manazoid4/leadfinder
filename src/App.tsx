import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';

type Lead = {
  id: number;
  businessName: string;
  trade: string;
  area: string;
  phone: string;
  website?: string;
  gapReason: string;
  confidence: string;
  eligible: boolean;
  opener: string;
  outcome?: string;
};

const outcomes = ['No answer', 'Interested', 'Callback', 'Booked', 'Not interested', 'Do not call'];

export default function App() {
  const [leads, setLeads] = useState<Lead[]>([]);
  const [selected, setSelected] = useState<Lead | null>(null);
  const [message, setMessage] = useState('');

  async function load() {
    const next = await invoke<Lead[]>('list_leads');
    setLeads(next);
    setSelected(current => next.find(lead => lead.id === current?.id) ?? next[0] ?? null);
  }

  useEffect(() => { void load(); }, []);

  async function saveOutcome(outcome: string) {
    if (!selected) return;
    await invoke('save_outcome', { id: selected.id, outcome });
    setMessage(`Saved: ${outcome}`);
    await load();
  }

  return (
    <main className="app-shell">
      <header className="topbar">
        <div><p className="eyebrow">MAZ WORKS · LOCAL WINDOWS WORKSTATION</p><h1>LEADFINDER</h1><p className="subtitle">Find leads. Check evidence. Make honest calls.</p></div>
        <div className="system-strip"><span>SQLite <b>● ready</b></span><span>Discovery <b>● CSV/Gosom</b></span><span>TPS <b>● key needed</b></span><span>9router <b>○ optional</b></span></div>
      </header>
      <section className="actions" aria-label="Lead actions">
        <button className="primary">FIND LEADS</button><button>IMPORT CSV</button><button onClick={() => selected && document.getElementById('call-view')?.scrollIntoView()}>START CALLING</button><button>CALLBACKS DUE <span className="count">{leads.filter(lead => lead.outcome === 'Callback').length}</span></button>
      </section>
      <div className="workspace">
        <aside className="lead-list"><div className="list-head"><div><p className="eyebrow">QUALIFICATION QUEUE</p><h2>{leads.length} lead{leads.length === 1 ? '' : 's'}</h2></div><span className="stage">SLICE 1</span></div>{leads.map(lead => <button className={`lead-row ${selected?.id === lead.id ? 'active' : ''}`} key={lead.id} onClick={() => setSelected(lead)}><span><strong>{lead.businessName}</strong><small>{lead.trade} · {lead.area}</small></span><span className={lead.eligible ? 'ready' : 'blocked'}>{lead.eligible ? 'READY' : 'BLOCKED'}</span></button>)}</aside>
        {selected && <section className="call-view" id="call-view"><div className="call-header"><div><p className="eyebrow">CALL VIEW · {selected.trade.toUpperCase()}</p><h2>{selected.businessName}</h2><p className="muted">{selected.area} · {selected.phone}</p></div><span className={selected.eligible ? 'status ready' : 'status blocked'}>{selected.eligible ? 'ELIGIBLE' : 'NOT ELIGIBLE'}</span></div><div className="evidence"><div><span className="label">GAP REASON</span><strong>{selected.gapReason}</strong></div><div><span className="label">CONFIDENCE</span><strong>{selected.confidence}</strong></div><div><span className="label">WEBSITE</span><strong>{selected.website ?? 'Not found'}</strong></div></div><article className="say-this"><p className="eyebrow">SAY THIS · DETERMINISTIC OPENER</p><p>{selected.opener}</p></article><article className="ai-brief"><p className="eyebrow">AI BRIEF — ADVISORY, DO NOT READ ALOUD</p><p>Optional advisory briefs will appear here after the deterministic calling flow is stable.</p></article><div className="outcomes"><p className="eyebrow">OUTCOME</p>{outcomes.map(outcome => <button key={outcome} disabled={!selected.eligible} onClick={() => void saveOutcome(outcome)}>{outcome}</button>)}</div>{message && <p className="saved" role="status">{message}</p>}</section>}
      </div>
    </main>
  );
}
