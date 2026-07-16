// Backend bridge. Inside the Tauri app these call the Rust commands; in a plain
// browser (e.g. `vite dev` for UI work) they fall back to mock data so the app renders.

import { invoke } from '@tauri-apps/api/core';
import type { ProjectSummary, ProjectDetail, UpdateInfo } from './types';

const inTauri =
  typeof window !== 'undefined' && '__TAURI_INTERNALS__' in window;

async function call<T>(cmd: string, args: Record<string, unknown>, mock: () => T): Promise<T> {
  if (!inTauri) return Promise.resolve(mock());
  return invoke<T>(cmd, args);
}

export const api = {
  isDemo: !inTauri,

  getRoots: () => call<string[]>('get_roots', {}, () => MOCK_ROOTS),

  addRoot: (path: string) =>
    call<string[]>('add_root', { path }, () => [...MOCK_ROOTS, path]),

  removeRoot: (path: string) =>
    call<string[]>('remove_root', { path }, () => MOCK_ROOTS.filter((r) => r !== path)),

  listProjects: () => call<ProjectSummary[]>('list_projects', {}, () => MOCK_PROJECTS),

  getProject: (path: string) =>
    call<ProjectDetail>('get_project', { path }, () => mockDetail(path)),

  exportReport: (path: string, subtitle?: string) =>
    call<string>('export_report', { path, subtitle }, () => `${path}/trackly-report.html`),

  openPath: (path: string) => call<void>('open_path', { path }, () => undefined),

  // Auto-update. Returns null when up to date; rejects if the updater isn't
  // configured yet (see UPDATER.md) — callers treat both as "nothing to do".
  checkUpdate: () => call<UpdateInfo | null>('check_update', {}, () => null),

  installUpdate: () => call<void>('install_update', {}, () => undefined),
};

// ---- mock data (demo mode only) ----

const MOCK_ROOTS = ['/Users/you/Dev'];

const MOCK_PROJECTS: ProjectSummary[] = [
  { path: '/Users/you/Dev/payments-api', name: 'payments-api', title: 'Acme Payments API', percent: 40.9, done: 4, partial: 1, open: 6, total: 11, branch: 'main' },
  { path: '/Users/you/Dev/trackly', name: 'trackly', title: 'Trackly', percent: 78.5, done: 12, partial: 2, open: 3, total: 17, branch: 'main' },
  { path: '/Users/you/Dev/storefront', name: 'storefront', title: 'Storefront Redesign', percent: 22.0, done: 2, partial: 1, open: 7, total: 10, branch: 'redesign' },
  { path: '/Users/you/Dev/ml-pipeline', name: 'ml-pipeline', title: 'Data Pipeline', percent: 100, done: 8, partial: 0, open: 0, total: 8, branch: 'main' },
];

function mockDetail(path: string): ProjectDetail {
  const summary = MOCK_PROJECTS.find((p) => p.path === path) ?? MOCK_PROJECTS[0];
  return {
    summary,
    source: 'plan.md',
    groups: [
      {
        name: 'Phase 1 — Foundations',
        percent: 75,
        tasks: [
          { id: 't1', title: 'Scaffold the service', status: 'done', date: '2026-07-02' },
          { id: 't2', title: 'Wire the Postgres schema', status: 'done', date: '2026-07-03' },
          { id: 't3', title: 'Auth middleware + API keys', status: 'done', date: '2026-07-05' },
          { id: 't4', title: 'CI pipeline (build + test)', status: 'inprogress', date: null },
        ],
      },
      {
        name: 'Phase 2 — Core flows',
        percent: 38,
        tasks: [
          { id: 't5', title: 'Create-charge endpoint', status: 'done', date: '2026-07-15' },
          { id: 't6', title: 'Refund endpoint', status: 'partial', date: null },
          { id: 't7', title: 'Webhook receiver', status: 'open', date: null },
          { id: 't8', title: 'Idempotency keys', status: 'open', date: null },
        ],
      },
      {
        name: 'Phase 3 — Hardening',
        percent: 0,
        tasks: [
          { id: 't9', title: 'Rate limiting', status: 'open', date: null },
          { id: 't10', title: 'Fraud screening (blocked on vendor keys)', status: 'blocked', date: null },
          { id: 't11', title: 'Load tests', status: 'open', date: null },
        ],
      },
    ],
    history: [
      { at: '2026-07-02 09:00', percent: 9 },
      { at: '2026-07-05 14:00', percent: 22 },
      { at: '2026-07-08 11:00', percent: 27 },
      { at: '2026-07-11 16:00', percent: 34 },
      { at: '2026-07-15 10:00', percent: 40.9 },
    ],
  };
}
