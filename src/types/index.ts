// src/types/index.ts

export enum Priority {
  A = 'A',
  B = 'B',
  C = 'C',
  D = 'D',
}

export interface Project {
  id: string;
  name: string;
  priority: Priority;
  logoPath?: string;
  createdAt: string;
  completionPercentage: number;
  timeSpent: number; // in hours
}

export interface WorkSession {
  id: string;
  projectId: string;
  projectName: string;
  goal: string;
  workDone: string[];
  startTime: string;
  endTime?: string;
  durationMinutes: number;
  isSynced: boolean;
}

export interface AppStats {
  activeProjects: number;
  todayTimeHours: number;
  productivityScore: number;
  goalsAchieved: number;
  totalGoals: number;
}

export interface TauriCommands {
  // Project commands
  get_projects: () => Promise<Project[]>;
  create_project: (project: Omit<Project, 'id' | 'createdAt'>) => Promise<Project>;
  update_project: (id: string, updates: Partial<Project>) => Promise<void>;
  delete_project: (id: string) => Promise<void>;
  
  // Session commands
  start_session: (projectId: string, goal: string) => Promise<WorkSession>;
  update_session: (sessionId: string, workDone: string) => Promise<void>;
  end_session: (sessionId: string) => Promise<WorkSession>;
  get_sessions: (projectId?: string) => Promise<WorkSession[]>;
  
  // Analytics
  get_analytics: (projectId: string) => Promise<{
    totalTimeMinutes: number;
    goalsSet: number;
    goalsAchieved: number;
    completionRate: number;
    productivityScore: number;
  }>;
  
  // Sync
  sync_to_cloud: () => Promise<void>;
  check_online: () => Promise<boolean>;
}