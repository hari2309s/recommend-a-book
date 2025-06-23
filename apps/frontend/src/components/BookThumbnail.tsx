import { motion } from 'framer-motion';
import type { FC } from 'react';

interface BookThumbnailProps {
  src?: string;
  alt: string;
  className?: string;
}

const BookThumbnail: FC<BookThumbnailProps> = ({ src, alt, className }) => {
  const isImageAvailable = src && src.trim() !== '';

  return isImageAvailable ? (
    <div className="relative w-36! h-48!">
      <motion.img
        src={src}
        alt={alt}
        className={className}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.3 }}
        style={{ aspectRatio: '3/4' }}
      />
    </div>
  ) : (
    <motion.div
      className={`relative overflow-hidden bg-accent-5 ${className} min-w-36! min-h-48!`}
      initial={{ opacity: 0 }}
      animate={{ opacity: 1 }}
      transition={{ duration: 0.3 }}
    >
      <div className="absolute inset-0 animate-shimmer bg-gradient-to-r from-accent-5 via-accent-6 to-accent-5 bg-[length:800px_100%]" />
    </motion.div>
  );
};

export default BookThumbnail;
