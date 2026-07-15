// src/components/Dashboard.tsx

import React, { useEffect, useMemo } from 'react';
import { useStore } from '../store/useStore';
import { Priority } from '../types';
import { Activity, Target, TrendingUp, CheckCircle2 } from 'lucide-react';

interface DashboardProps {
  onNewProject: () => void;
  onStartWork: () => void;
}

export const Dashboard: React.FC<DashboardProps> = ({ onNewProject, onStartWork }) => {
  const { projects, sessions, loadSessions } = useStore();

  useEffect(() => {
    loadSessions();
  }, [loadSessions]);

  const stats = useMemo(() => {
    const today = new Date().toDateString();
    const todaySessions = sessions.filter(s => 
      new Date(s.startTime).toDateString() === today
    );

    const totalTimeToday = todaySessions.reduce((sum, s) => sum + s.durationMinutes, 0);
    const goalsAchieved = todaySessions.filter(s => s.workDone.length > 0).length;
    
    const avgCompletion = projects.length > 0
      ? projects.reduce((sum, p) => sum + p.completionPercentage, 0) / projects.length
      : 0;

    return {
      activeProjects: projects.length,
      todayTimeHours: (totalTimeToday / 60).toFixed(1),
      productivityScore: Math.round(avgCompletion),
      goalsAchieved,
      totalGoals: todaySessions.length,
    };
  }, [projects, sessions]);

  const getPriorityColor = (priority: Priority): string => {
    const colors = {
      [Priority.A]: 'from-red-500 to-red-600',
      [Priority.B]: 'from-orange-500 to-orange-600',
      [Priority.C]: 'from-green-500 to-green-600',
      [Priority.D]: 'from-slate-500 to-slate-600',
    };
    return colors[priority];
  };

  return (
    <div className="dashboard">
      {/* Header */}
      <div className="header">
        <div className="logo-container">
          <h1 className="logo">Trackly</h1>
          <p className="tagline">Your productivity, measured and mastered</p>
        </div>
      </div>

      {/* Stats Grid */}
      <div className="stats-grid">
        <div className="stat-card">
          <div className="stat-icon">
            <Target size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Active Projects</span>
            <span className="stat-value">{stats.activeProjects}</span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <Activity size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Today's Focus Time</span>
            <span className="stat-value">{stats.todayTimeHours}<span className="stat-unit">h</span></span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <TrendingUp size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Productivity Score</span>
            <span className="stat-value">{stats.productivityScore}<span className="stat-unit">%</span></span>
          </div>
        </div>

        <div className="stat-card">
          <div className="stat-icon">
            <CheckCircle2 size={24} />
          </div>
          <div className="stat-content">
            <span className="stat-label">Goals Achieved</span>
            <span className="stat-value">{stats.goalsAchieved}<span className="stat-unit">/{stats.totalGoals}</span></span>
          </div>
        </div>
      </div>

      {/* Projects Section */}
      <div className="glass-card">
        <div className="section-header">
          <h2 className="section-title">Your Projects</h2>
          <button className="btn btn-primary" onClick={onNewProject}>
            + New Project
          </button>
        </div>

        {projects.length === 0 ? (
          <div className="empty-state">
            <div className="empty-state-icon">📊</div>
            <h3>No projects yet</h3>
            <p>Create your first project to start tracking your productivity</p>
            <button className="btn btn-primary" onClick={onNewProject}>
              Create Project
            </button>
          </div>
        ) : (
          <div className="projects-table-container">
            <table className="projects-table">
              <thead>
                <tr>
                  <th>Project</th>
                  <th>Priority</th>
                  <th>Progress</th>
                  <th>Time Spent</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                {projects.map(project => (
                  <tr key={project.id}>
                    <td>
                      <div className="project-name">{project.name}</div>
                    </td>
                    <td>
                      <span className={`priority-badge priority-${project.priority}`}>
                        {project.priority}
                      </span>
                    </td>
                    <td>
                      <div className="progress-container">
                        <div className="progress-bar-bg">
                          <div 
                            className="progress-bar-fill"
                            style={{ width: `${project.completionPercentage}%` }}
                          />
                        </div>
                        <span className="progress-text">{project.completionPercentage}%</span>
                      </div>
                    </td>
                    <td>{project.timeSpent.toFixed(1)}h</td>
                    <td>
                      <button 
                        className="btn btn-secondary btn-sm"
                        onClick={onStartWork}
                      >
                        Track
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
};