import { motion } from 'framer-motion';
import type { FC } from 'react';
import imageNotAvailable from '@/assets/image-not-available.png';

interface BookThumbnailProps {
  src?: string;
  alt: string;
  className?: string;
}

const BookThumbnail: FC<BookThumbnailProps> = ({ src, alt, className }) => {
  const isImageAvailable = src && src.trim() !== '';

  return isImageAvailable ? (
    <div className="w-36! h-48!">
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
    <motion.div className="w-36! h-48!">
      <motion.img src={imageNotAvailable} alt="Image Not Available" className={className}
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.3 }}
        style={{ aspectRatio: '3/4' }} />
    </motion.div>
  );
};

export default BookThumbnail;
