import { motion } from 'framer-motion';

interface ProgressRingProps {
  percentage: number;
  size?: number;
  strokeWidth?: number;
  label: string;
  color?: string;
  backgroundColor?: string;
  delay?: number;
}

export function ProgressRing({
  percentage,
  size = 120,
  strokeWidth = 8,
  label,
  color = '#22c55e',
  backgroundColor = '#222230',
  delay = 0,
}: ProgressRingProps) {
  const radius = (size - strokeWidth) / 2;
  const circumference = radius * 2 * Math.PI;
  const offset = circumference - (percentage / 100) * circumference;

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.8 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.5, delay }}
      className="flex flex-col items-center"
    >
      <div className="relative" style={{ width: size, height: size }}>
        <svg width={size} height={size} className="absolute inset-0">
          {/* Background circle */}
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke={backgroundColor}
            strokeWidth={strokeWidth}
          />

          {/* Progress circle */}
          <motion.circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="none"
            stroke={color}
            strokeWidth={strokeWidth}
            strokeDasharray={circumference}
            strokeDashoffset={offset}
            strokeLinecap="round"
            initial={{ strokeDashoffset: circumference }}
            animate={{ strokeDashoffset: offset }}
            transition={{ duration: 1.5, delay: delay + 0.2, ease: 'easeOut' }}
            style={{
              transform: 'rotate(-90deg)',
              transformOrigin: `${size / 2}px ${size / 2}px`,
              filter: `drop-shadow(0 0 4px ${color}20)`,
            }}
          />
        </svg>

        {/* Center text */}
        <div className="absolute inset-0 flex flex-col items-center justify-center">
          <motion.span
            className="text-2xl font-bold text-white"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5, delay: delay + 0.1 }}
          >
            {percentage}%
          </motion.span>
        </div>
      </div>

      <p className="mt-2 text-xs font-medium uppercase tracking-wider text-gray-400">
        {label}
      </p>
    </motion.div>
  );
}
