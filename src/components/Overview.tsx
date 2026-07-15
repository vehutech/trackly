import type { ProjectSummary } from '../types';

function Ring({ percent }: { percent: number }) {
  const r = 26;
  const c = 2 * Math.PI * r;
  const off = c * (1 - percent / 100);
  return (
    <svg className="ring" viewBox="0 0 64 64" width="64" height="64">
      <circle cx="32" cy="32" r={r} className="ring-track" />
      <circle
        cx="32"
        cy="32"
        r={r}
        className="ring-fill"
        strokeDasharray={c}
        strokeDashoffset={off}
        transform="rotate(-90 32 32)"
      />
      <text x="32" y="36" className="ring-text">{percent.toFixed(0)}%</text>
    </svg>
  );
}

interface Props {
  projects: ProjectSummary[];
  onSelect: (path: string) => void;
}

export function Overview({ projects, onSelect }: Props) {
  const totals = projects.reduce(
    (a, p) => ({ done: a.done + p.done, total: a.total + p.total, projects: a.projects + 1 }),
    { done: 0, total: 0, projects: 0 },
  );

  return (
    <div className="overview">
      <div className="ov-head">
        <h1>Your projects</h1>
        <p className="ov-sub">
          {totals.projects} tracked {totals.projects === 1 ? 'repo' : 'repos'} · {totals.done}/{totals.total} items done
        </p>
      </div>

      {projects.length === 0 ? (
        <div className="empty-state">
          <div className="empty-emoji">◔</div>
          <h2>No tracked repos yet</h2>
          <p>
            Run <code>trackly init</code> in a repo, then add its parent folder here with
            <b> Add folder</b> and press <b>Rescan</b>.
          </p>
        </div>
      ) : (
        <div className="card-grid">
          {projects.map((p) => (
            <button key={p.path} className="ov-card" onClick={() => onSelect(p.path)}>
              <Ring percent={p.percent} />
              <div className="ov-card-body">
                <div className="ov-card-title">{p.title}</div>
                <div className="ov-card-meta">
                  {p.name}
                  {p.branch ? ` · ${p.branch}` : ''}
                </div>
                <div className="ov-card-stats">
                  <span className="s-done">{p.done} done</span>
                  <span className="s-part">{p.partial} partial</span>
                  <span className="s-open">{p.open} open</span>
                </div>
              </div>
            </button>
          ))}
        </div>
      )}
    </div>
  );
}
