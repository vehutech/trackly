// src/store/useStore.ts

import { create } from 'zustand';
import { Project, WorkSession, Priority } from '../types';
import { invoke } from '@tauri-apps/api/core';

interface AppState {
  // State
  projects: Project[];
  currentSession: WorkSession | null;
  sessions: WorkSession[];
  isOnline: boolean;
  
  // Actions
  loadProjects: () => Promise<void>;
  createProject: (name: string, priority: Priority) => Promise<void>;
  updateProject: (id: string, updates: Partial<Project>) => Promise<void>;
  deleteProject: (id: string) => Promise<void>;
  
  startSession: (projectId: string, goal: string) => Promise<void>;
  addWorkDone: (workDone: string) => Promise<void>;
  endSession: () => Promise<void>;
  
  loadSessions: (projectId?: string) => Promise<void>;
  syncToCloud: () => Promise<void>;
  checkOnlineStatus: () => Promise<void>;
}

export const useStore = create<AppState>((set, get) => ({
  projects: [],
  currentSession: null,
  sessions: [],
  isOnline: false,

  loadProjects: async () => {
    try {
      const projects = await invoke<Project[]>('get_projects');
      set({ projects });
    } catch (error) {
      console.error('Failed to load projects:', error);
    }
  },

  createProject: async (name: string, priority: Priority) => {
    try {
      const project = await invoke<Project>('create_project', {
        project: { name, priority, completionPercentage: 0, timeSpent: 0 }
      });
      set(state => ({ projects: [...state.projects, project] }));
    } catch (error) {
      console.error('Failed to create project:', error);
    }
  },

  updateProject: async (id: string, updates: Partial<Project>) => {
    try {
      await invoke('update_project', { id, updates });
      set(state => ({
        projects: state.projects.map(p => 
          p.id === id ? { ...p, ...updates } : p
        )
      }));
    } catch (error) {
      console.error('Failed to update project:', error);
    }
  },

  deleteProject: async (id: string) => {
    try {
      await invoke('delete_project', { id });
      set(state => ({
        projects: state.projects.filter(p => p.id !== id)
      }));
    } catch (error) {
      console.error('Failed to delete project:', error);
    }
  },

  startSession: async (projectId: string, goal: string) => {
    try {
      const session = await invoke<WorkSession>('start_session', {
        projectId,
        goal
      });
      set({ currentSession: session });
    } catch (error) {
      console.error('Failed to start session:', error);
    }
  },

  addWorkDone: async (workDone: string) => {
    const { currentSession } = get();
    if (!currentSession) return;

    try {
      await invoke('update_session', {
        sessionId: currentSession.id,
        workDone
      });
      set(state => ({
        currentSession: state.currentSession
          ? {
              ...state.currentSession,
              workDone: [...state.currentSession.workDone, workDone]
            }
          : null
      }));
    } catch (error) {
      console.error('Failed to add work done:', error);
    }
  },

  endSession: async () => {
    const { currentSession } = get();
    if (!currentSession) return;

    try {
      const completedSession = await invoke<WorkSession>('end_session', {
        sessionId: currentSession.id
      });
      
      set(state => ({
        currentSession: null,
        sessions: [...state.sessions, completedSession]
      }));
      
      // Reload projects to update completion percentages
      get().loadProjects();
    } catch (error) {
      console.error('Failed to end session:', error);
    }
  },

  loadSessions: async (projectId?: string) => {
    try {
      const sessions = await invoke<WorkSession[]>('get_sessions', { projectId });
      set({ sessions });
    } catch (error) {
      console.error('Failed to load sessions:', error);
    }
  },

  syncToCloud: async () => {
    try {
      await invoke('sync_to_cloud');
      console.log('Successfully synced to cloud');
    } catch (error) {
      console.error('Failed to sync to cloud:', error);
    }
  },

  checkOnlineStatus: async () => {
    try {
      const isOnline = await invoke<boolean>('check_online');
      set({ isOnline });
    } catch (error) {
      set({ isOnline: false });
    }
  },
}));