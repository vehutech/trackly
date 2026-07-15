// src/components/NewProjectModal.tsx

import React, { useState } from 'react';
import { useStore } from '../store/useStore';
import { Priority } from '../types';
import { X } from 'lucide-react';

interface NewProjectModalProps {
  isOpen: boolean;
  onClose: () => void;
}

export const NewProjectModal: React.FC<NewProjectModalProps> = ({ isOpen, onClose }) => {
  const { createProject } = useStore();
  const [name, setName] = useState('');
  const [priority, setPriority] = useState<Priority>(Priority.B);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!name.trim()) return;

    await createProject(name.trim(), priority);
    setName('');
    setPriority(Priority.B);
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Create New Project</h2>
          <button className="close-btn" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label className="form-label">Project Name</label>
            <input
              type="text"
              className="form-input"
              placeholder="e.g., Website Redesign"
              value={name}
              onChange={e => setName(e.target.value)}
              autoFocus
              required
            />
          </div>

          <div className="form-group">
            <label className="form-label">Priority Level</label>
            <div className="priority-selector">
              {Object.values(Priority).map(p => (
                <button
                  key={p}
                  type="button"
                  className={`priority-option priority-${p} ${priority === p ? 'selected' : ''}`}
                  onClick={() => setPriority(p)}
                >
                  <div className="priority-letter">{p}</div>
                  <div className="priority-desc">
                    {p === Priority.A && 'Critical'}
                    {p === Priority.B && 'High'}
                    {p === Priority.C && 'Medium'}
                    {p === Priority.D && 'Low'}
                  </div>
                </button>
              ))}
            </div>
          </div>

          <div className="modal-actions">
            <button type="button" className="btn btn-secondary" onClick={onClose}>
              Cancel
            </button>
            <button type="submit" className="btn btn-primary">
              Create Project
            </button>
          </div>
        </form>
      </div>
    </div>
  );
};

// src/components/WorkSessionModal.tsx

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

// src/components/ProgressCheckModal.tsx

interface ProgressCheckModalProps {
  isOpen: boolean;
  onClose: () => void;
  onSwitch: () => void;
}

export const ProgressCheckModal: React.FC<ProgressCheckModalProps> = ({ 
  isOpen, 
  onClose, 
  onSwitch 
}) => {
  const { currentSession, addWorkDone, endSession } = useStore();
  const [workDone, setWorkDone] = useState('');
  const [showWorkInput, setShowWorkInput] = useState(false);

  const handleContinue = () => {
    setShowWorkInput(true);
  };

  const handleSaveProgress = async () => {
    if (workDone.trim()) {
      await addWorkDone(workDone.trim());
      setWorkDone('');
      setShowWorkInput(false);
      onClose();
    }
  };

  const handleEndSession = async () => {
    if (workDone.trim()) {
      await addWorkDone(workDone.trim());
    }
    await endSession();
    setWorkDone('');
    setShowWorkInput(false);
    onClose();
  };

  if (!isOpen || !currentSession) return null;

  const elapsedMinutes = Math.floor(
    (Date.now() - new Date(currentSession.startTime).getTime()) / 60000
  );

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" onClick={e => e.stopPropagation()}>
        <div className="modal-header">
          <h2>Still working on {currentSession.projectName}?</h2>
          <button className="close-btn" onClick={onClose}>
            <X size={20} />
          </button>
        </div>

        <div className="timer-display">
          Active for: {elapsedMinutes} minutes
        </div>

        <div className="session-goal">
          <p className="goal-label">Your Goal:</p>
          <p className="goal-text">{currentSession.goal}</p>
        </div>

        {!showWorkInput ? (
          <div className="modal-actions">
            <button className="btn btn-secondary" onClick={onSwitch}>
              Switch Project
            </button>
            <button className="btn btn-primary" onClick={handleContinue}>
              Yes, Continue
            </button>
          </div>
        ) : (
          <>
            <div className="form-group">
              <label className="form-label">What have you accomplished?</label>
              <textarea
                className="form-textarea"
                placeholder="Describe what you've completed..."
                value={workDone}
                onChange={e => setWorkDone(e.target.value)}
                rows={4}
                autoFocus
              />
            </div>

            {currentSession.workDone.length > 0 && (
              <div className="work-done-list">
                <p className="work-done-label">Previous accomplishments:</p>
                {currentSession.workDone.map((work, idx) => (
                  <div key={idx} className="work-done-item">
                    {work}
                  </div>
                ))}
              </div>
            )}

            <div className="modal-actions">
              <button className="btn btn-secondary" onClick={handleEndSession}>
                End Session
              </button>
              <button className="btn btn-primary" onClick={handleSaveProgress}>
                Save & Continue
              </button>
            </div>
          </>
        )}
      </div>
    </div>
  );
};