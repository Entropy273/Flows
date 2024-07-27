import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import AppUsageComponent from './AppUsageComponent';
import { listen } from '@tauri-apps/api/event';
import { register, unregisterAll } from '@tauri-apps/api/globalShortcut';

type AppUsage = {
  [key: string]: [number, number][];
};

const App: React.FC = () => {
  const [appUsages, setAppUsages] = useState<AppUsage>({});

  useEffect(() => {
    async function registerGlobalShortcut() {
      try {
        await unregisterAll();
        await register('Option+F', async () => {
          await invoke('show_window');
        });
      } catch (error) {
        console.error('Failed to register global shortcut:', error);
      }
    }
    registerGlobalShortcut();

    async function fetchAppUsages() {
      try {
        const usages: AppUsage = await invoke('get_app_usages');
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

  const getDurationFromUsages = (usages: AppUsage): [number, number] => {
    const start = Math.min(...Object.values(usages).map((times) => Math.min(...times.map(([start, _]) => start))));
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