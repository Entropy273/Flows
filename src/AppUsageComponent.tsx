import React from 'react';
import { AppUsage } from './App.tsx';

type AppUsageComponentProps = {
  appUsages: AppUsage[];
  startTime: number;
  endTime: number;
};

const AppUsageComponent: React.FC<AppUsageComponentProps> = ({ appUsages, startTime, endTime }) => {
  const colors = [
    'bg-[#F8E629]',
    'bg-[#9DBBD8]',
    'bg-[#B28F18]',
    'bg-[#0E50CA]',
    'bg-[#EEEEEF]',
    'bg-[#EE8505]',
    'bg-[#EFCFB3]'
  ];

  return (
    <div className="flex flex-row w-full h-full gap-3 dark:bg-gray-900 dark:text-white select-none">
      <div className="relative h-full w-1/2">
        {Array.from({ length: 13 }).map((_, i) => (
          <div
            key={i}
            className="absolute left-0 w-full border-t border-dashed border-gray-300 dark:border-gray-700"
            style={{ top: `${(i * 100) / 12}%` }}
          >
            <span className="absolute text-right w-10 transform -translate-y-1/2 bg-white dark:bg-gray-900 px-1 text-xs">
              {`${i * 2}:00`}
            </span>
          </div>
        ))}

        {appUsages.map(({name, durations}, index) => (
          <div key={name} className="absolute left-10 right-0 h-full">
            {durations.map(([start, end], i) => {
              const top = `${(start - startTime) / (endTime - startTime) * 100}%`;
              const height = `${(end - start) / (endTime - startTime) * 100}%`;

              return (
                <div
                  key={i}
                  className={`absolute w-full ${colors[index % colors.length]} opacity-75`}
                  style={{
                    top: top,
                    height: height,
                  }}
                ></div>
              );
            })}
          </div>
        ))}
      </div>
      <div className='w-1/2 p-2 overflow-y-auto'>
        {appUsages.map(({ name, total_secs: totalSecs }, index) => (
          <div key={name} className="mb-4 p-4 bg-white dark:bg-gray-800 shadow rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition duration-300 flex flex-col items-start">
            <div className='flex flex-row justify-start items-center'>
              <span className={`w-2 h-2 rounded-full ${colors[index % colors.length]} mr-2`}></span>
              <h3 className="font-bold">{name}</h3>
            </div>
            <div>
              <p>{totalSecs >= 3600 ? `${Math.floor(totalSecs / 3600)}h` : ''} {totalSecs >= 60 ? `${Math.floor((totalSecs % 3600) / 60)}m` : ''} {`${totalSecs % 60}s`}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default AppUsageComponent;