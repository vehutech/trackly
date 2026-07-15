// Shapes returned by the Rust backend (see src-tauri/src/lib.rs).

export interface ProjectSummary {
  path: string;
  name: string;
  title: string;
  percent: number;
  done: number;
  partial: number;
  open: number;
  total: number;
  branch?: string | null;
}

export interface TaskView {
  id: string;
  title: string;
  status: 'open' | 'inprogress' | 'partial' | 'done' | 'blocked' | string;
  date?: string | null;
}

export interface GroupView {
  name: string;
  percent: number;
  tasks: TaskView[];
}

export interface SnapshotView {
  at: string;
  percent: number;
}

export interface ProjectDetail {
  summary: ProjectSummary;
  source?: string | null;
  groups: GroupView[];
  history: SnapshotView[];
}
