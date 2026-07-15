// src/components/ProgressCheckModal.tsx

import React, { useState } from 'react';
import { useStore } from '../store/useStore';
import { X } from 'lucide-react';

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