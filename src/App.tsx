import { useEffect, useMemo, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import './App.css';
import DemoPage from './DemoPage';

const stages = ['New', 'Research', 'Qualified', 'Demo', 'Contacted', 'Replied', 'Won', 'Lost', 'Follow-up'];
const outcomes = ['No answer', 'Interested', 'Callback', 'Booked', 'Not interested', 'Do not call'];
const templates = [
  { id: 'shopify-engraving-preview', name: 'Shopify engraving preview', vertical: 'UK personalised-gift Shopify stores', discovery: 'Web search', offer: '£150 install' },
  { id: 'local-engraving-preview', name: 'Local engraving preview', vertical: 'Masons, trophies, signage, embroidery', discovery: 'Gosom', offer: '£150 setup' },
];

type Lead = {
  id: number; businessName: string; trade: string; area: string; phone?: string; website?: string;
  contactChannel: string; status: string; nextAction?: string; opportunity?: string; solution?: string;
  templateId?: string; demoUrl?: string; gapReason: string; confidence: string; eligible: boolean;
  opener: string; outcome?: string; verificationCount: number;
};
type LeadEditor = Pick<Lead, 'status' | 'nextAction' | 'opportunity' | 'solution' | 'templateId' | 'demoUrl'>;
type ModelStatus = { routerReady: boolean };
type DatabaseHealth = { ready: boolean; schemaVersion: number };
type ImportSummary = { imported: number; deduplicated: number; rejected: number; errors: string[] };
type SiteSignals = { technologies: string[]; rejectFingerprints: string[]; verdict: string; reason: string; statusCode: number };
type Partner = { id: number; name: string; contactName?: string; email?: string; phone?: string; referredLeads: number; conversions: number; notes: string };

function fixedOutreach(demoUrl?: string) {
  if (!demoUrl) return 'Generate and save a real demo link before outreach.';
  return `Hi — built you something, no pitch attached. Buyers can't see their engraving before they pay on your site. So I made a working version: ${demoUrl} Open it on your phone, type a name. I'm Maz — I do this setup for UK stores end to end, so you don't have to pick through apps and wire it up. £150 to put it live properly. If it's not for you, keep the link.`;
}

function Workstation() {
  const [view, setView] = useState<'pipeline' | 'templates' | 'partners'>('pipeline');
  const [leads, setLeads] = useState<Lead[]>([]);
  const [selected, setSelected] = useState<Lead | null>(null);
  const [editor, setEditor] = useState<LeadEditor>({ status: 'New' });
  const [partners, setPartners] = useState<Partner[]>([]);
  const [partnerName, setPartnerName] = useState('');
  const [partnerContact, setPartnerContact] = useState('');
  const [partnerNotes, setPartnerNotes] = useState('');
  const [message, setMessage] = useState('');
  const [trade, setTrade] = useState('personalised engraved gifts');
  const [area, setArea] = useState('UK');
  const [modelStatus, setModelStatus] = useState<ModelStatus | null>(null);
  const [databaseHealth, setDatabaseHealth] = useState<DatabaseHealth | null>(null);
  const [busy, setBusy] = useState(false);
  const [smartReview, setSmartReview] = useState('');

  async function load() {
    const next = await invoke<Lead[]>('list_leads');
    setLeads(next);
    const chosen = next.find(lead => lead.id === selected?.id) ?? next[0] ?? null;
    setSelected(chosen);
    if (chosen) setEditor({ status: chosen.status, nextAction: chosen.nextAction, opportunity: chosen.opportunity, solution: chosen.solution, templateId: chosen.templateId, demoUrl: chosen.demoUrl });
  }
  async function loadPartners() { setPartners(await invoke<Partner[]>('list_partners')); }
  /* oxlint-disable react/set-state-in-effect, react-hooks/exhaustive-deps -- initial Tauri I/O */
  useEffect(() => {
    void load().catch(error => setMessage(`Lead storage unavailable: ${String(error)}`));
    void loadPartners().catch(error => setMessage(`Partner storage unavailable: ${String(error)}`));
    void invoke<DatabaseHealth>('database_health').then(setDatabaseHealth).catch(() => setDatabaseHealth(null));
    void invoke<ModelStatus>('model_status').then(setModelStatus).catch(() => setModelStatus(null));
  }, []);
  /* oxlint-enable react/set-state-in-effect, react-hooks/exhaustive-deps */
  function chooseLead(lead: Lead) {
    setSelected(lead);
    setEditor({ status: lead.status, nextAction: lead.nextAction, opportunity: lead.opportunity, solution: lead.solution, templateId: lead.templateId, demoUrl: lead.demoUrl });
    setSmartReview('');
  }

  const stageCounts = useMemo(() => Object.fromEntries(stages.map(stage => [stage, leads.filter(lead => lead.status === stage).length])), [leads]);
  function summary(result: ImportSummary) {
    const errorText = result.errors.length ? ` ${result.errors.length} row error(s): ${result.errors.slice(0, 2).join(' | ')}` : '';
    return `Imported ${result.imported}; deduplicated ${result.deduplicated}; rejected ${result.rejected}.${errorText}`;
  }
  async function discover(kind: 'maps' | 'web') {
    setBusy(true); setMessage(kind === 'maps' ? 'Planning bounded Maps searches through 9router…' : 'Searching the public web without a model…');
    try {
      const result = kind === 'maps'
        ? await invoke<ImportSummary>('discover_leads', { queries: await invoke<string[]>('plan_search', { trade, area }) })
        : await invoke<ImportSummary>('discover_web', { query: `${trade} ${area}` });
      setMessage(`${summary(result)} New leads start at 2/5 evidence passes.`); await load();
    } catch (error) { setMessage(`${kind === 'maps' ? 'Maps' : 'Web'} discovery failed loudly: ${String(error)}`); }
    finally { setBusy(false); }
  }
  async function importCsv(event: React.ChangeEvent<HTMLInputElement>) {
    const file = event.target.files?.[0]; if (!file) return; setBusy(true);
    try { const result = await invoke<ImportSummary>('import_csv', { contents: await file.text() }); setMessage(summary(result)); await load(); }
    catch (error) { setMessage(`CSV import failed: ${String(error)}`); }
    finally { setBusy(false); event.target.value = ''; }
  }
  async function research() {
    if (!selected) return; setBusy(true); setMessage('Running deterministic technology and reject-fingerprint checks…');
    try { const result = await invoke<SiteSignals>('research_lead', { id: selected.id }); setMessage(`${result.verdict}: ${result.reason}. HTTP ${result.statusCode}; ${result.technologies.join(', ') || 'no technologies found'}.`); await load(); }
    catch (error) { setMessage(`Research failed; lead remains unqualified: ${String(error)}`); }
    finally { setBusy(false); }
  }
  async function saveLead() {
    if (!selected) return;
    try { await invoke('update_lead', { id: selected.id, update: editor }); setMessage('Lead pipeline record saved.'); await load(); }
    catch (error) { setMessage(`Lead not saved: ${String(error)}`); }
  }
  async function saveOutcome(outcome: string) {
    if (!selected) return;
    try { await invoke('save_outcome', { id: selected.id, outcome }); setMessage(`Saved contact outcome: ${outcome}`); await load(); }
    catch (error) { setMessage(`Outcome not saved: ${String(error)}`); }
  }
  async function runSmartReview() {
    if (!selected) return; setSmartReview('9router is reviewing captured evidence…');
    try { setSmartReview(await invoke<string>('smart_review', { businessName: selected.businessName, website: selected.website ?? null, evidence: `${selected.gapReason}; verification=${selected.verificationCount}/5` })); }
    catch (error) { setSmartReview(`Smart review unavailable: ${String(error)}`); }
  }
  async function addPartner() {
    try { await invoke('create_partner', { name: partnerName, contactName: partnerContact || null, email: null, phone: null, notes: partnerNotes }); setPartnerName(''); setPartnerContact(''); setPartnerNotes(''); await loadPartners(); setMessage('Partner saved.'); }
    catch (error) { setMessage(`Partner not saved: ${String(error)}`); }
  }

  return <main className="app-shell">
    <header className="topbar"><div><p className="eyebrow">MAZ WORKS · MANUAL OUTREACH WORKSTATION</p><h1>LEADFINDER</h1><p className="subtitle">Find → research → demo → contact → track.</p></div><div className="system-strip"><span>SQLite <b className={databaseHealth?.ready ? 'ready' : 'blocked'}>{databaseHealth?.ready ? `● schema ${databaseHealth.schemaVersion}` : '○ unavailable'}</b></span><span>Discovery <b>● Web + Gosom</b></span><span>Models <b className={modelStatus?.routerReady ? 'ready' : 'blocked'}>{modelStatus?.routerReady ? '● 9router' : '○ unavailable'}</b></span><span>Sending <b>● manual only</b></span></div></header>
    <nav className="tabs">{(['pipeline', 'templates', 'partners'] as const).map(tab => <button className={view === tab ? 'active' : ''} onClick={() => setView(tab)} key={tab}>{tab.toUpperCase()}</button>)}</nav>
    {view === 'pipeline' && <>
      <section className="pipeline-strip">{stages.map(stage => <button key={stage}><span>{stage}</span><strong>{stageCounts[stage]}</strong></button>)}</section>
      <section className="actions" aria-label="Lead actions"><input aria-label="Vertical or trade" value={trade} onChange={event => setTrade(event.target.value)} /><input aria-label="Area" value={area} onChange={event => setArea(event.target.value)} /><button className="primary" disabled={busy} onClick={() => void discover('web')}>WEB DISCOVERY</button><button disabled={busy} onClick={() => void discover('maps')}>GOSOM DISCOVERY</button><label className="file-button">IMPORT CSV<input type="file" accept=".csv,text/csv" onChange={event => void importCsv(event)} /></label></section>
      <div className="workspace"><aside className="lead-list"><div className="list-head"><div><p className="eyebrow">LEAD QUEUE</p><h2>{leads.length} lead{leads.length === 1 ? '' : 's'}</h2></div><span className="stage">5-PASS GATE</span></div>{leads.length === 0 && <p className="empty">No fake seed data. Run discovery or import real leads.</p>}{leads.map(lead => <button className={`lead-row ${selected?.id === lead.id ? 'active' : ''}`} key={lead.id} onClick={() => chooseLead(lead)}><span><strong>{lead.businessName}</strong><small>{lead.status} · {lead.area}</small></span><span className={lead.eligible ? 'ready' : 'blocked'}>{lead.eligible ? 'READY' : `${lead.verificationCount}/5`}</span></button>)}</aside>
        {selected && <section className="call-view"><div className="call-header"><div><p className="eyebrow">COMPANY · CONTACTS · EVIDENCE</p><h2>{selected.businessName}</h2><p className="muted">{selected.phone ?? 'No phone'} · {selected.website ?? 'No website'} · {selected.contactChannel.toUpperCase()}</p></div><span className={selected.eligible ? 'status ready' : 'status blocked'}>{selected.eligible ? 'CONTACT UNLOCKED' : 'CONTACT LOCKED'}</span></div>
          <div className="evidence"><div><span className="label">STATUS</span><strong>{selected.status}</strong></div><div><span className="label">OPPORTUNITY</span><strong>{selected.opportunity ?? 'Not identified'}</strong></div><div><span className="label">TEMPLATE</span><strong>{selected.templateId ?? 'Not selected'}</strong></div><div><span className="label">VERIFICATION</span><strong>{selected.verificationCount}/5</strong></div></div>
          <div className="record-grid"><label>Status<select value={editor.status} onChange={event => setEditor({ ...editor, status: event.target.value })}>{stages.map(stage => <option key={stage}>{stage}</option>)}</select></label><label>Next action<input value={editor.nextAction ?? ''} onChange={event => setEditor({ ...editor, nextAction: event.target.value })} /></label><label>Opportunity<input value={editor.opportunity ?? ''} onChange={event => setEditor({ ...editor, opportunity: event.target.value })} /></label><label>Solution<input value={editor.solution ?? ''} onChange={event => setEditor({ ...editor, solution: event.target.value })} /></label><label>Template<select value={editor.templateId ?? ''} onChange={event => setEditor({ ...editor, templateId: event.target.value || undefined })}><option value="">Not selected</option>{templates.map(template => <option key={template.id} value={template.id}>{template.name}</option>)}</select></label><label>Demo link<input value={editor.demoUrl ?? ''} onChange={event => setEditor({ ...editor, demoUrl: event.target.value })} /></label><div className="record-actions"><button disabled={busy || !selected.website} onClick={() => void research()}>RESEARCH SITE</button><button className="primary" onClick={() => void saveLead()}>SAVE LEAD</button>{editor.demoUrl && <button onClick={() => void navigator.clipboard.writeText(editor.demoUrl ?? '')}>COPY DEMO</button>}</div></div>
          <article className="say-this"><p className="eyebrow">FIXED OUTREACH — NO MODEL COPY</p><p>{selected.opener || fixedOutreach(editor.demoUrl)}</p></article>
          <article className="ai-brief"><p className="eyebrow">ADVISORY REVIEW</p><button onClick={() => void runSmartReview()}>RUN 9ROUTER REVIEW</button><p>{smartReview || selected.gapReason}</p></article>
          <div className="outcomes"><p className="eyebrow">MANUAL CONTACT OUTCOME</p>{outcomes.map(outcome => <button key={outcome} disabled={!selected.eligible} onClick={() => void saveOutcome(outcome)}>{outcome}</button>)}</div>
        </section>}
      </div>
    </>}
    {view === 'templates' && <section className="cards"><div className="section-head"><p className="eyebrow">START WITH TWO</p><h2>Problem templates</h2></div>{templates.map(template => <article className="template-card" key={template.id}><p className="stage">{template.discovery}</p><h2>{template.name}</h2><p>{template.vertical}</p><strong>{template.offer}</strong>{template.id === 'shopify-engraving-preview' && <a href="#/demo/rfid-wallets-uk">Open real RFID Wallets UK demo</a>}<small>Audit + solution + demo + fixed outreach. Config-driven; another lead needs data, not component code.</small></article>)}</section>}
    {view === 'partners' && <section className="partners"><div className="section-head"><p className="eyebrow">NOT A PARTNER PORTAL</p><h2>Partners and referrals</h2></div><div className="partner-form"><input placeholder="Partner name" value={partnerName} onChange={event => setPartnerName(event.target.value)} /><input placeholder="Contact" value={partnerContact} onChange={event => setPartnerContact(event.target.value)} /><input placeholder="Notes" value={partnerNotes} onChange={event => setPartnerNotes(event.target.value)} /><button className="primary" onClick={() => void addPartner()}>ADD PARTNER</button></div><div className="partner-table">{partners.length === 0 ? <p className="empty">No partners yet.</p> : partners.map(partner => <article key={partner.id}><strong>{partner.name}</strong><span>{partner.contactName ?? 'No contact'}</span><span>{partner.referredLeads} referred · {partner.conversions} won</span><small>{partner.notes}</small></article>)}</div></section>}
    {message && <p className="saved global-message" role="status">{message}</p>}
  </main>;
}

export default function App() {
  const demoSlug = window.location.hash.match(/^#\/demo\/([^/?#]+)/)?.[1];
  return demoSlug ? <DemoPage slug={decodeURIComponent(demoSlug)} /> : <Workstation />;
}
