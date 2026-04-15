import { motion } from 'framer-motion';
import { Card, CardHeader, CardTitle } from './Card';

interface HeatmapCell {
  date: string;
  activity: number; // 0-4 intensity
  label: string;
}

interface ActivityHeatmapProps {
  data: HeatmapCell[];
  title?: string;
  delay?: number;
}

export function ActivityHeatmap({
  data,
  title = '12-Week Activity',
  delay = 0,
}: ActivityHeatmapProps) {
  const weeks = [];
  for (let i = 0; i < 12; i++) {
    weeks.push(data.filter((_, idx) => Math.floor(idx / 7) === i));
  }

  const getColor = (activity: number): string => {
    if (activity === 0) return 'bg-surface-2';
    if (activity === 1) return 'bg-accent-green/30';
    if (activity === 2) return 'bg-accent-green/60';
    if (activity === 3) return 'bg-accent-green/85';
    return 'bg-accent-green';
  };

  const dayLabels = ['Mon', 'Wed', 'Fri'];

  return (
    <Card delay={delay}>
      <CardHeader>
        <CardTitle>{title}</CardTitle>
      </CardHeader>

      <div className="overflow-x-auto">
        <div className="flex gap-1 pb-4 min-w-max">
          {/* Y-axis labels */}
          <div className="flex flex-col gap-1 pt-6 pr-2">
            {dayLabels.map((day) => (
              <span key={day} className="text-[10px] font-medium text-gray-600 h-3">
                {day}
              </span>
            ))}
          </div>

          {/* Heatmap grid */}
          <div className="flex gap-1">
            {weeks.map((week, weekIdx) => (
              <div key={weekIdx} className="flex flex-col gap-1">
                {week.length < 7
                  ? Array.from({ length: 7 }, (_, i) => {
                    const cell = week[i];
                    return (
                      <motion.div
                        key={`${weekIdx}-${i}`}
                        initial={{ opacity: 0, scale: 0.8 }}
                        animate={{ opacity: 1, scale: 1 }}
                        transition={{
                          duration: 0.3,
                          delay: delay + (weekIdx + i) * 0.01,
                        }}
                        className={`h-3 w-3 rounded-sm border border-surface-3 ${
                          cell ? getColor(cell.activity) : 'bg-surface-2'
                        } cursor-help transition-all hover:ring-2 hover:ring-accent-green/50`}
                        title={cell?.label || ''}
                      />
                    );
                  })
                  : week.map((cell, i) => (
                    <motion.div
                      key={`${weekIdx}-${i}`}
                      initial={{ opacity: 0, scale: 0.8 }}
                      animate={{ opacity: 1, scale: 1 }}
                      transition={{
                        duration: 0.3,
                        delay: delay + (weekIdx + i) * 0.01,
                      }}
                      className={`h-3 w-3 rounded-sm border border-surface-3 ${getColor(
                        cell.activity
                      )} cursor-help transition-all hover:ring-2 hover:ring-accent-green/50`}
                      title={cell.label}
                    />
                  ))}
              </div>
            ))}
          </div>
        </div>

        {/* Legend */}
        <div className="flex items-center gap-2 pt-2 text-[10px] text-gray-600">
          <span>Less</span>
          {[0, 1, 2, 3, 4].map((level) => (
            <div
              key={level}
              className={`h-2 w-2 rounded-sm ${
                level === 0
                  ? 'bg-surface-2'
                  : level === 1
                    ? 'bg-accent-green/30'
                    : level === 2
                      ? 'bg-accent-green/60'
                      : level === 3
                        ? 'bg-accent-green/85'
                        : 'bg-accent-green'
              }`}
            />
          ))}
          <span>More</span>
        </div>
      </div>
    </Card>
  );
}
