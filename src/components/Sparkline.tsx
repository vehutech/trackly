import type { SnapshotView } from '../types';

// A small inline-SVG trend line of completion over time. No chart library needed.
export function Sparkline({ history }: { history: SnapshotView[] }) {
  const w = 520;
  const h = 96;
  const pad = 6;

  if (history.length < 2) {
    return <div className="spark-empty">Not enough history yet — it fills in as the plan changes.</div>;
  }

  const xs = history.map((_, i) => (i / (history.length - 1)) * (w - pad * 2) + pad);
  const ys = history.map((s) => h - pad - (s.percent / 100) * (h - pad * 2));
  const line = xs.map((x, i) => `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${ys[i].toFixed(1)}`).join(' ');
  const area = `${line} L${xs[xs.length - 1].toFixed(1)},${h - pad} L${xs[0].toFixed(1)},${h - pad} Z`;
  const last = history[history.length - 1];

  return (
    <svg className="spark" viewBox={`0 0 ${w} ${h}`} preserveAspectRatio="none" role="img" aria-label="progress over time">
      <path d={area} className="spark-area" />
      <path d={line} className="spark-line" />
      <circle cx={xs[xs.length - 1]} cy={ys[ys.length - 1]} r={3.5} className="spark-dot" />
      <title>{`${last.percent.toFixed(1)}% as of ${last.at}`}</title>
    </svg>
  );
}
