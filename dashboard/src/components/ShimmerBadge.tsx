import { motion } from 'framer-motion';
import './shimmer.css';

interface ShimmerBadgeProps {
  count: number;
  label: string;
  color?: 'gold' | 'green' | 'blue' | 'purple';
}

export function ShimmerBadge({ count, label, color = 'gold' }: ShimmerBadgeProps) {
  const colorClass = {
    gold: 'from-amber-400 to-amber-600',
    green: 'from-accent-green to-green-600',
    blue: 'from-brand-400 to-brand-600',
    purple: 'from-accent-purple to-purple-600',
  }[color];

  const textColor = {
    gold: 'text-amber-950',
    green: 'text-green-950',
    blue: 'text-blue-950',
    purple: 'text-purple-950',
  }[color];

  return (
    <motion.div
      initial={{ opacity: 0, scale: 0.8 }}
      animate={{ opacity: 1, scale: 1 }}
      transition={{ duration: 0.5 }}
      className="relative inline-block"
    >
      {/* Shimmer background */}
      <div className={`absolute inset-0 bg-gradient-to-r ${colorClass} rounded-full blur opacity-75 group-hover:opacity-100 transition duration-500`} />

      {/* Badge */}
      <div className={`relative px-4 py-2 bg-gradient-to-r ${colorClass} rounded-full`}>
        <div className="shimmer absolute inset-0 rounded-full" />

        <div className={`relative flex items-center gap-2 font-bold ${textColor}`}>
          <span className="text-lg">{count}</span>
          <span className="text-xs uppercase tracking-wider">{label}</span>
        </div>
      </div>
    </motion.div>
  );
}
