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

const App: React.FC = () => {
  const [appUsages, setAppUsages] = useState<AppUsage[]>([]);

  useEffect(() => {
    document.addEventListener('contextmenu', (event) => {
      event.preventDefault();
    });

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
    registerGlobalShortcut();

    async function fetchAppUsages() {
      try {
        const usages: AppUsage[] = await invoke('get_app_usages_handler');
        console.log("App usage:\n" + usages.map(usage => 
          `${usage.name}: ${usage.total_secs}s, ${usage.durations.length} times`
        ).join('\n'));
        setAppUsages(usages);
      } catch (error) {
        console.error('Failed to fetch app usages:', error);
      }
    }
    fetchAppUsages();

    const unlisten = listen('refresh_data', () => {
      fetchAppUsages();
    });

    return () => {
      unlisten.then((off) => off());
    };
  }, []);

  const getDurationFromUsages = (usages: AppUsage[]): [number, number] => {
    const start = Math.min(...usages.map((usage) => usage.durations[0][0]));
    const date = new Date(start);
    date.setHours(0, 0, 0, 0);
    const startTime = date.getTime();
    date.setHours(24, 0, 0, 0);
    const endTime = date.getTime();
    return [startTime, endTime];
  };

  const [startTime, endTime] = getDurationFromUsages(appUsages);

  return (
    <div className="p-10 h-screen justify-around dark:bg-gray-900">
        <AppUsageComponent appUsages={appUsages} startTime={startTime} endTime={endTime} />
    </div>
  );
};

export default App;