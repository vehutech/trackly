// src/App.tsx

import React, { useEffect, useState } from 'react';
import { useStore } from './store/useStore';
import { Dashboard } from './components/Dashboard';
import { NewProjectModal } from './components/NewProjectModal';
import { WorkSessionModal } from './components/WorkSessionModal';
import { ProgressCheckModal } from './components/ProgressCheckModal';
import { listen } from '@tauri-apps/api/event';
import './App.css';

const App: React.FC = () => {
  const { loadProjects, checkOnlineStatus, currentSession } = useStore();
  const [showNewProject, setShowNewProject] = useState(false);
  const [showWorkSession, setShowWorkSession] = useState(false);
  const [showProgressCheck, setShowProgressCheck] = useState(false);

  useEffect(() => {
    // Load initial data
    loadProjects();
    checkOnlineStatus();

    // Check online status every minute
    const onlineInterval = setInterval(checkOnlineStatus, 60000);

    // Listen for activity prompts from Rust backend
    const unlisten = listen('prompt_user', (event) => {
      const data = event.payload as { type: string };
      
      if (data.type === 'start_session' && !currentSession) {
        setShowWorkSession(true);
      } else if (data.type === 'check_progress' && currentSession) {
        setShowProgressCheck(true);
      }
    });

    return () => {
      clearInterval(onlineInterval);
      unlisten.then(fn => fn());
    };
  }, [loadProjects, checkOnlineStatus, currentSession]);

  return (
    <div className="app">
      <Dashboard 
        onNewProject={() => setShowNewProject(true)}
        onStartWork={() => setShowWorkSession(true)}
      />

      <NewProjectModal
        isOpen={showNewProject}
        onClose={() => setShowNewProject(false)}
      />

      <WorkSessionModal
        isOpen={showWorkSession}
        onClose={() => setShowWorkSession(false)}
      />

      <ProgressCheckModal
        isOpen={showProgressCheck}
        onClose={() => setShowProgressCheck(false)}
        onSwitch={() => {
          setShowProgressCheck(false);
          setShowWorkSession(true);
        }}
      />
    </div>
  );
};

export default App;