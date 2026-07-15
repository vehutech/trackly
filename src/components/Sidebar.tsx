import type { ProjectSummary } from '../types';

interface Props {
  projects: ProjectSummary[];
  selected: string | null;
  onSelect: (path: string) => void;
  onHome: () => void;
  loading: boolean;
}

export function Sidebar({ projects, selected, onSelect, onHome, loading }: Props) {
  return (
    <aside className="sidebar">
      <button className="brand" onClick={onHome} title="Overview">
        <span className="brand-dot" /> Trackly
      </button>

      <div className="side-label">
        Projects {projects.length > 0 && <span className="count">{projects.length}</span>}
      </div>

      <nav className="project-list">
        {loading && <div className="side-empty">Scanning…</div>}
        {!loading && projects.length === 0 && <div className="side-empty">No tracked repos found.</div>}
        {projects.map((p) => (
          <button
            key={p.path}
            className={`project-item ${selected === p.path ? 'active' : ''}`}
            onClick={() => onSelect(p.path)}
          >
            <div className="pi-top">
              <span className="pi-name">{p.title}</span>
              <span className="pi-pct">{p.percent.toFixed(0)}%</span>
            </div>
            <div className="pi-bar">
              <div className="pi-fill" style={{ width: `${p.percent}%` }} />
            </div>
          </button>
        ))}
      </nav>
    </aside>
  );
}
