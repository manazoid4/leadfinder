import { useEffect, useRef, useState } from 'react';
import { createEngravingPreview } from '../render-gate/engraving.js';
import './DemoPage.css';

type DemoConfig = {
  slug: string;
  template: string;
  company: string;
  ownerFirstName: string | null;
  productName: string;
  productUrl: string;
  productImageUrl: string;
  price: string;
  accentHex: string;
  engraving: {
    x: number;
    y: number;
    maxWidth: number;
    colour: [number, number, number];
    maxCharacters: number;
  };
};

export default function DemoPage({ slug }: { slug: string }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const previewRef = useRef<{ render(text: string): void } | null>(null);
  const [config, setConfig] = useState<DemoConfig | null>(null);
  const [text, setText] = useState('MAZ');
  const [status, setStatus] = useState('Loading demo…');

  useEffect(() => {
    document.body.classList.add('demo-mode');
    const robots = document.querySelector<HTMLMetaElement>('meta[name="robots"]') ?? document.head.appendChild(document.createElement('meta'));
    robots.name = 'robots';
    robots.content = 'noindex,nofollow';
    fetch(`/demo-configs/${encodeURIComponent(slug)}.json`)
      .then(response => {
        if (!response.ok) throw new Error(`Demo config returned ${response.status}`);
        return response.json() as Promise<DemoConfig>;
      })
      .then(setConfig)
      .catch(error => setStatus(`Demo unavailable: ${String(error)}`));
    return () => document.body.classList.remove('demo-mode');
  }, [slug]);

  useEffect(() => {
    if (!config || !canvasRef.current) return;
    document.title = `${config.company} — live engraving preview`;
    setStatus('Loading product image…');
    void createEngravingPreview(canvasRef.current, {
      productImageUrl: config.productImageUrl,
      engraving: config.engraving,
    })
      .then(preview => {
        previewRef.current = preview;
        preview.render('MAZ');
        setStatus('Preview ready — type a name.');
      })
      .catch(error => setStatus(`Preview unavailable: ${String(error)}`));
  }, [config]);

  function updateText(value: string) {
    const next = value.slice(0, config?.engraving.maxCharacters ?? 16);
    setText(next);
    previewRef.current?.render(next);
  }

  async function copyLink() {
    try {
      await navigator.clipboard.writeText(window.location.href);
      setStatus('Demo link copied.');
    } catch {
      setStatus('Copy failed — select the browser address instead.');
    }
  }

  return <main className="demo-page" style={{ '--demo-accent': config?.accentHex ?? '#a06b19' } as React.CSSProperties}>
    <header className="demo-header">
      <div><p className="demo-eyebrow">Live personalisation concept for {config?.company ?? 'store'}</p><h1>See your engraving before checkout.</h1></div>
      {config && <strong className="demo-price">{config.price}</strong>}
    </header>
    <section className="demo-stage">
      <div className="demo-canvas-wrap"><canvas ref={canvasRef} aria-label="Personalised product preview" /></div>
      <div className="demo-controls">
        <p className="demo-eyebrow">Try it now</p>
        <label>Engraving text<input value={text} maxLength={config?.engraving.maxCharacters ?? 16} onChange={event => updateText(event.target.value)} /></label>
        <p>Preview uses the store's real product image. Placement and finish stay configurable during installation.</p>
        <p className="demo-status" role="status">{status}</p>
        <button type="button" onClick={() => void copyLink()}>Copy demo link</button>
        {config && <a href={config.productUrl} rel="noreferrer" target="_blank">View original product</a>}
      </div>
    </section>
  </main>;
}
