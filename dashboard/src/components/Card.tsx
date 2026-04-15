import { motion } from 'framer-motion';
import { clsx } from 'clsx';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  delay?: number;
}

export function Card({ children, className, delay = 0 }: CardProps) {
  return (
    <motion.div
      initial={{ opacity: 0, y: 12 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.4, delay, ease: 'easeOut' }}
      className={clsx(
        'relative rounded-xl border border-surface-3 bg-surface-1 p-5 shadow-lg shadow-black/20 overflow-hidden',
        className
      )}
    >
      {/* Gradient accent top-right */}
      <div className="absolute -right-12 -top-12 h-40 w-40 rounded-full bg-gradient-to-br from-brand-500/10 to-accent-purple/10 blur-3xl" />

      <div className="relative z-10">{children}</div>
    </motion.div>
  );
}

export function CardHeader({ children, className }: { children: React.ReactNode; className?: string }) {
  return (
    <div className={clsx('mb-4 flex items-center gap-2', className)}>
      {children}
    </div>
  );
}

export function CardTitle({ children }: { children: React.ReactNode }) {
  return <h3 className="text-sm font-semibold uppercase tracking-wider text-gray-400">{children}</h3>;
}
