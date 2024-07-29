import React, { useEffect, useRef, useState } from 'react';
import { AppUsage } from './App.tsx';

type AppUsageComponentProps = {
  appUsages: AppUsage[];
  startTimestamp: number;
  endTimestamp: number;
};

const AppUsageComponent: React.FC<AppUsageComponentProps> = ({ appUsages, startTimestamp, endTimestamp }) => {
  const colors = [
    'bg-[#F8E629]',
    'bg-[#9DBBD8]',
    'bg-[#B28F18]',
    'bg-[#0E50CA]',
    'bg-[#EEEEEF]',
    'bg-[#EE8505]',
    'bg-[#EFCFB3]'
  ];

  const [cardExpanded, setCardExpanded] = useState<{ [key: string]: boolean }>({});
  const [heights, setHeights] = useState<{ [key: string]: number }>({});
  const [selectedApps, setSelectedApps] = useState<string[]>([]);
  const [mousePosition, setMousePosition] = useState<number>(0);
  const [mouseLineState, setMouseLineState] = useState<boolean>(false);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const cardRefs = useRef<{ [key: string]: HTMLDivElement | null }>({});

  const toggleExpand = (name: string) => {
    setCardExpanded(prev => ({
      ...prev,
      [name]: !prev[name],
    }));
  };

  const handleMouseEnter = (name: string) => {
    setSelectedApps(prev => [...prev, name]);
  };

  const handleMouseLeave = (name: string) => {
    setTimeout(() => {
      setSelectedApps(prev => prev.filter(app => app !== name));
    }, 70);
  };

  useEffect(() => {
    const newHeights: { [key: string]: number } = {};
    Object.keys(cardExpanded).forEach((name) => {
      if (cardRefs.current[name]) {
        newHeights[name] = cardRefs.current[name]!.scrollHeight;
      }
    });
    setHeights(newHeights);
  }, [cardExpanded]);

  const calculateTimeLabel = (position: number) => {
    const containerHeight = containerRef.current?.getBoundingClientRect().height || window.innerHeight;
    const timePercentage = position / containerHeight;
    const timestamp = startTimestamp + timePercentage * (endTimestamp - startTimestamp);
    const date = new Date(timestamp);
    return `${date.getHours().toString().padStart(2, '0')}:${date.getMinutes().toString().padStart(2, '0')}`;
  };

  return (
    <div className="flex flex-row w-full flex-grow pt-2 pb-5 overflow-y-auto gap-3 dark:bg-gray-900 dark:text-white select-none">
      {/** App usage chart */}
      <div
        ref={containerRef} 
        className="relative h-full w-1/2"
        onMouseMove={(e) => {
          const containerTop = e.currentTarget.getBoundingClientRect().top;
          setMousePosition(e.clientY - containerTop);
          setMouseLineState(true);
        }}
        onMouseLeave={() => {
          setMouseLineState(false);
        }}
      >
        {/** Time scale lines */}
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

        {/** App usage blocks */}
        {appUsages.map(({ name, durations }, index) => (
          <div key={name} className="absolute left-10 right-0 h-full">
            {durations.map(([start, end], i) => {
              const top = `${((start - startTimestamp) / (endTimestamp - startTimestamp)) * 100}%`;
              const height = `${((end - start) / (endTimestamp - startTimestamp)) * 100}%`;

              return (
                <div
                  key={i}
                  className={`absolute w-full transition-all duration-500 ease-in-out ${colors[index % colors.length]} ${selectedApps.length > 0 && !selectedApps.includes(name) ? 'opacity-5' : 'opacity-90'}`}
                  style={{
                    top: top,
                    height: height,
                  }}
                ></div>
              );
            })}
          </div>
        ))}

        {/** Mouse hover line and time label */}
          <div
            className={`absolute left-0 w-full border-t border-red-500 shadow-lg shadow-red-500 transition duration-500 ${mouseLineState ? 'opacity-100' : 'opacity-0'}`}
            style={{ top: mousePosition }}
          >
            <span className="absolute text-right w-10 transform -translate-y-1/2 bg-white dark:bg-gray-900 px-1 text-xs">
              {calculateTimeLabel(mousePosition)}
            </span>
          </div>
      </div>

      {/** App usage card list */}
      <div className='w-1/2 p-2 overflow-y-auto'>
        {appUsages.map(({ name, path, total_secs: totalSecs }, index) => (
          <div 
            key={name} 
            className="mb-4 p-4 bg-white dark:bg-gray-800 shadow rounded hover:bg-gray-200 dark:hover:bg-gray-700 transition duration-300 flex flex-col items-start"
            onMouseEnter={() => handleMouseEnter(name)}
            onMouseLeave={() => handleMouseLeave(name)}
          >
            <div className='flex flex-row justify-start items-center w-full'>
              <span className={`w-2 h-2 rounded-full ${colors[index % colors.length]} mr-2`}></span>
              <h3 className="font-bold flex-grow">{name}</h3>
              <button className={`ml-auto transform transition-transform duration-300 ${cardExpanded[name] ? '-rotate-90' : ''}`} onClick={() => toggleExpand(name)}>
                <img src="LeftArrow.svg" alt="expand" className="w-5 h-5" />
              </button>
            </div>
            <div>
              <p>{totalSecs >= 3600 ? `${Math.floor(totalSecs / 3600)}h` : ''} {totalSecs >= 60 ? `${Math.floor((totalSecs % 3600) / 60)}m` : ''} {`${totalSecs % 60}s`}</p>
            </div>
            <div
              ref={el => (cardRefs.current[name] = el)}
              className={`w-full break-all text-xs transition-all duration-300 ease-in-out overflow-hidden`}
              style={{
                maxHeight: cardExpanded[name] ? `${heights[name]}px` : '0',
                opacity: cardExpanded[name] ? '0.3' : '0',
              }}
            >
              {path}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
};

export default AppUsageComponent;