import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import AppUsageComponent from './AppUsageComponent';
import { listen } from '@tauri-apps/api/event';
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';

export type AppUsage = {
  name: string,
  path: string,
  total_secs: number,
  durations: [number, number][],
};

function getTodayStartTimestamp(): number {
  const now = new Date();
  return (new Date(now.getFullYear(), now.getMonth(), now.getDate())).getTime();
}

const App: React.FC = () => {
  const [appUsages, setAppUsages] = useState<AppUsage[]>([]);
  const [startTimestamp, setStartTimestamp] = useState<number>(getTodayStartTimestamp());
  const [endTimestamp, setEndTimestamp] = useState<number>(getTodayStartTimestamp() + 86399000);

  async function registerGlobalShortcut() {
    try {
      await unregisterAll();
      await register('Option+F', async () => {
        await invoke('show_window_handler');
      });
    } catch (error) {
      console.error('Failed to register global shortcut:', error);
    }
  }

  async function fetchAppUsages(_startTimestamp: number, _endTimestamp: number) {
    try {
      const usages: AppUsage[] = await invoke('get_app_usages_handler', {startTimestamp: _startTimestamp, endTimestamp: _endTimestamp});
      console.log("App usage:\n" + usages.map(usage => 
        `${usage.name}: ${usage.total_secs}s, ${usage.durations.length} times`
      ).join('\n'));
      setAppUsages(usages);
    } catch (error) {
      console.error('Failed to fetch app usages:', error);
    }
  }

  useEffect(() => {
    document.addEventListener('contextmenu', (event) => {
      event.preventDefault();
    });
    
    registerGlobalShortcut();

    fetchAppUsages(startTimestamp, endTimestamp);

    const unlisten = listen('refresh_data', () => {
      fetchAppUsages(startTimestamp, endTimestamp);
    });

    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const handlePrevDay = () => {
    fetchAppUsages(startTimestamp - 86400000, endTimestamp - 86400000);
    setStartTimestamp((startTimestamp) => startTimestamp - 86400000);
    setEndTimestamp((endTimestamp) => endTimestamp - 86400000);
  };

  const handleNextDay = () => {
    fetchAppUsages(startTimestamp + 86400000, endTimestamp + 86400000);
    setStartTimestamp((startTimestamp) => startTimestamp + 86400000);
    setEndTimestamp((endTimestamp) => endTimestamp + 86400000);
  };

  const formatDate = (date: Date): string => {
    return date.toISOString().split('T')[0];
  };

  return (
    <div className="flex flex-col gap-3 p-5 pb-20 h-screen justify-start dark:bg-gray-900">
      <div className="flex justify-between items-center">
        <button onClick={handlePrevDay} className="text-white-800 dark:text-white p-2 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition duration-300"><img src="LeftArrow.svg" alt="expand" className="w-5 h-5" /></button>
        <span className="text-xl dark:text-white">{formatDate(new Date(startTimestamp))}</span>
        <button onClick={handleNextDay} className="text-white-800 dark:text-white p-2 rounded hover:bg-gray-300 dark:hover:bg-gray-600 transition duration-300"><img src="LeftArrow.svg" alt="expand" className="w-5 h-5 rotate-180" /></button>
      </div>
      <AppUsageComponent appUsages={appUsages} 
        startTimestamp={startTimestamp} 
        endTimestamp={endTimestamp} />
    </div>
  );
};

export default App;