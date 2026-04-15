import { useEffect, useRef } from 'react';
import { motion, useSpring, useTransform } from 'framer-motion';

interface AnimatedCounterProps {
  value: number;
  duration?: number;
  precision?: number;
  prefix?: string;
  suffix?: string;
  className?: string;
}

export function AnimatedCounter({
  value,
  duration = 1.5,
  precision = 0,
  prefix = '',
  suffix = '',
  className = '',
}: AnimatedCounterProps) {
  const springValue = useSpring(0, {
    duration: duration * 1000,
    bounce: 0,
  });

  const displayValue = useTransform(springValue, (v) => {
    const decimals = precision > 0 ? v.toFixed(precision) : Math.floor(v).toString();
    return `${prefix}${decimals}${suffix}`;
  });

  const prevValueRef = useRef(0);

  useEffect(() => {
    if (value !== prevValueRef.current) {
      springValue.set(value);
      prevValueRef.current = value;
    }
  }, [value, springValue]);

  return (
    <motion.span className={className}>
      {displayValue}
    </motion.span>
  );
}
