import { useEffect, useState } from 'react';
import { api } from '../api';
import type { UpdateInfo } from '../types';

// Shows a bar when a newer signed build is available. Silent (renders nothing)
// when up to date, in demo mode, or when the updater isn't configured yet.
export function UpdateBanner() {
  const [update, setUpdate] = useState<UpdateInfo | null>(null);
  const [installing, setInstalling] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.checkUpdate().then(setUpdate).catch(() => {
      // Updater not configured / offline — nothing to show.
    });
  }, []);

  if (!update) return null;

  async function install() {
    setInstalling(true);
    setError(null);
    try {
      await api.installUpdate(); // app restarts on success
    } catch (e) {
      setInstalling(false);
      setError(String(e));
    }
  }

  return (
    <div className="update-banner">
      <span>
        <b>Trackly {update.version}</b> is available.
        {error && <span className="update-err"> {error}</span>}
      </span>
      <button className="btn" onClick={install} disabled={installing}>
        {installing ? 'Downloading…' : 'Update & restart'}
      </button>
    </div>
  );
}
