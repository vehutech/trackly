import { useCallback, useEffect, useState } from 'react';
import './styles.css';
import { api } from './api';
import type { ProjectSummary } from './types';
import { Sidebar } from './components/Sidebar';
import { Overview } from './components/Overview';
import { ProjectView } from './components/ProjectView';
import { UpdateBanner } from './components/UpdateBanner';

export default function App() {
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [roots, setRoots] = useState<string[]>([]);
  const [selected, setSelected] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [adding, setAdding] = useState(false);
  const [newRoot, setNewRoot] = useState('');

  const rescan = useCallback(async () => {
    setLoading(true);
    try {
      const list = await api.listProjects();
      setProjects(list);
      setSelected((cur) => (cur && list.some((p) => p.path === cur) ? cur : null));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    api.getRoots().then(setRoots);
    rescan();
  }, [rescan]);

  async function addRoot() {
    const path = newRoot.trim();
    if (!path) return;
    const updated = await api.addRoot(path);
    setRoots(updated);
    setNewRoot('');
    setAdding(false);
    rescan();
  }

  async function dropRoot(path: string) {
    setRoots(await api.removeRoot(path));
    rescan();
  }

  return (
    <div className="app">
      <Sidebar
        projects={projects}
        selected={selected}
        onSelect={setSelected}
        onHome={() => setSelected(null)}
        loading={loading}
      />

      <main className="content">
        <UpdateBanner />
        <div className="topbar">
          <div className="roots">
            {roots.map((r) => (
              <span className="root-chip" key={r} title={r}>
                {shorten(r)}
                <button className="chip-x" onClick={() => dropRoot(r)} title="Remove folder">×</button>
              </span>
            ))}
          </div>

          <div className="actions">
            {adding ? (
              <div className="add-row">
                <input
                  autoFocus
                  className="add-input"
                  placeholder="/path/to/folder"
                  value={newRoot}
                  onChange={(e) => setNewRoot(e.target.value)}
                  onKeyDown={(e) => e.key === 'Enter' && addRoot()}
                />
                <button className="btn" onClick={addRoot}>Add</button>
                <button className="btn ghost" onClick={() => setAdding(false)}>Cancel</button>
              </div>
            ) : (
              <button className="btn ghost" onClick={() => setAdding(true)}>+ Add folder</button>
            )}
            <button className="btn ghost" onClick={rescan} disabled={loading}>
              {loading ? 'Scanning…' : '↻ Rescan'}
            </button>
          </div>
        </div>

        {api.isDemo && (
          <div className="demo-banner">
            Demo data — run inside the Trackly desktop app to see your real repos.
          </div>
        )}

        {selected ? <ProjectView path={selected} /> : <Overview projects={projects} onSelect={setSelected} />}
      </main>
    </div>
  );
}

function shorten(path: string): string {
  const parts = path.split('/').filter(Boolean);
  return parts.length <= 2 ? path : `…/${parts.slice(-2).join('/')}`;
}
