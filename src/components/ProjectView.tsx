import { useEffect, useState } from 'react';
import type { ProjectDetail } from '../types';
import { api } from '../api';
import { Sparkline } from './Sparkline';
import { statusClass, statusGlyph } from './status';

export function ProjectView({ path }: { path: string }) {
  const [detail, setDetail] = useState<ProjectDetail | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [exportMsg, setExportMsg] = useState<string | null>(null);

  useEffect(() => {
    let alive = true;
    setDetail(null);
    setError(null);
    setExportMsg(null);
    api
      .getProject(path)
      .then((d) => alive && setDetail(d))
      .catch((e) => alive && setError(String(e)));
    return () => {
      alive = false;
    };
  }, [path]);

  if (error) return <div className="pv-error">Couldn’t load project: {error}</div>;
  if (!detail) return <div className="pv-loading">Loading…</div>;

  const { summary, groups, history, source } = detail;

  async function exportReport() {
    try {
      const out = await api.exportReport(path, summary.title);
      await api.openPath(out);
      setExportMsg(`Report written to ${out}`);
    } catch (e) {
      setExportMsg(`Export failed: ${String(e)}`);
    }
  }

  return (
    <div className="project-view">
      <header className="pv-head">
        <div>
          <h1>{summary.title}</h1>
          <p className="pv-meta">
            {summary.name}
            {summary.branch ? ` · ${summary.branch}` : ''}
            {source ? ` · ${source}` : ''}
          </p>
        </div>
        <button className="btn primary" onClick={exportReport}>Export report</button>
      </header>

      {exportMsg && <div className="pv-note">{exportMsg}</div>}

      <section className="tiles">
        <div className="tile pct"><div className="n">{summary.percent.toFixed(1)}%</div><div className="l">overall</div></div>
        <div className="tile done"><div className="n">{summary.done}</div><div className="l">done</div></div>
        <div className="tile partial"><div className="n">{summary.partial}</div><div className="l">partial</div></div>
        <div className="tile open"><div className="n">{summary.open}</div><div className="l">open</div></div>
      </section>

      <section className="trend">
        <div className="section-label">Progress over time</div>
        <Sparkline history={history} />
      </section>

      <section className="detail">
        <div className="section-label">Tasks</div>
        {groups.map((g) => (
          <div key={g.name} className="group">
            <div className="group-head">
              <span className="gname">{g.name}</span>
              <span className="gpct">{g.percent.toFixed(0)}%</span>
            </div>
            {g.tasks.map((t) => (
              <div key={t.id} className={`item ${statusClass(t.status)}`}>
                <span className="dot">{statusGlyph(t.status)}</span>
                <span className="tid">{t.id}</span>
                <span className="txt">{t.title}</span>
                {t.date && <span className="date">{t.date}</span>}
              </div>
            ))}
          </div>
        ))}
      </section>
    </div>
  );
}
