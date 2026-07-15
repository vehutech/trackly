// src/components/WorkSessionModal.tsx

import React, { useState } from 'react';
import { useStore } from '../store/useStore';
import { X } from 'lucide-react';

interface WorkSessionModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const WorkSessionModal: React.FC<WorkSessionModalProps> = ({ isOpen, onClose }) => {
  const { projects, startSession } = useStore();
  const [projectId, setProjectId] = useState('');
  const [goal, setGoal] = useState('');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!projectId || !goal.trim()) return;

    await startSession(projectId, goal.trim());
    setProjectId('');
    setGoal('');
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>What are you working on?</h2>
          <button className="close-btn" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label className="form-label">Select Project</label>
            <select
              className="form-select"
              value={projectId}
              onChange={e => setProjectId(e.target.value)}
              required
            >
              <option value="">Choose a project...</option>
              {projects.map(project => (
                <option key={project.id} value={project.id}>
                  {project.name} ({project.priority})
                </option>
              ))}
            </select>
          </div>

          <div className="form-group">
            <label className="form-label">What's your goal for the next hour?</label>
            <textarea
              className="form-textarea"
              placeholder="e.g., Complete homepage layout"
              value={goal}
              onChange={e => setGoal(e.target.value)}
              rows={4}
              required
            />
          </div>

          <div className="modal-actions">
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              Later
            </button>
            <button type="submit" className="btn btn-primary">
              Start Tracking
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};