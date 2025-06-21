import { motion } from 'framer-motion';
import type { FC } from 'react';

interface ThumbnailWithSkeletonProps {
  src?: string;
  alt: string;
  className?: string;
}

const ThumbnailWithSkeleton: FC<ThumbnailWithSkeletonProps> = ({ src, alt, className }) => {
  const isImageAvailable = src && src.trim() !== '';

  return isImageAvailable ? (
    <motion.img
      src={src}
      alt={alt}
      className={className}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.3 }}
    />
  ) : (
    <motion.div
      className={`relative overflow-hidden bg-slate-500 ${className}`}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.3 }}
    >
      <div className="absolute inset-0 animate-shimmer bg-gradient-to-r from-slate-500 via-green-200 to-slate-500 bg-[length:800px_100%]" />
    </motion.div>
  );
};

export default ThumbnailWithSkeleton;
