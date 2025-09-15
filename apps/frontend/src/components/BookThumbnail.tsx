import { motion } from 'framer-motion';
import React from 'react';
import { useState } from 'react';
import imageNotAvailable from '@/assets/image-not-available.png';
import { thumbnailVariants } from '@/utils/animations';

type BookThumbnailProps = {
  src?: string;
  alt: string;
  className?: string;
};

const BookThumbnail: React.FC<BookThumbnailProps> = ({
  src,
  alt,
  className,
}: BookThumbnailProps) => {
  const [imageLoaded, setImageLoaded] = useState<boolean>(false);
  const [imageError, setImageError] = useState<boolean>(false);
  const isImageAvailable: boolean = (src && src.trim() !== '') as boolean;

  const handleImageLoad = () => {
    setImageLoaded(true);
  };

  const handleImageError = () => {
    setImageError(true);
  };

  return (
    <motion.div
      className="w-36! h-48! relative overflow-hidden rounded-md"
      whileHover={{
        scale: 1.05,
        transition: { type: 'spring', stiffness: 300, damping: 20 },
      }}
      initial="initial"
      animate="animate"
      variants={thumbnailVariants}
    >
      {isImageAvailable && !imageError ? (
        <>
          {!imageLoaded && (
            <motion.div
              className="absolute inset-0 bg-gray-200 animate-pulse rounded-md"
              initial={{ opacity: 1 }}
              animate={{ opacity: imageLoaded ? 0 : 1 }}
              transition={{ duration: 0.3 }}
            />
          )}

          <motion.img
            src={src}
            alt={alt}
            className={`${className} rounded-md transition-opacity duration-300 ${
              imageLoaded ? 'opacity-100' : 'opacity-0'
            }`}
            style={{ aspectRatio: '3/4' }}
            onLoad={handleImageLoad}
            onError={handleImageError}
            initial={{ opacity: 0, y: 10 }}
            animate={{
              opacity: imageLoaded ? 1 : 0,
              y: imageLoaded ? 0 : 10,
            }}
            transition={{
              duration: 0.4,
              delay: 0.1,
            }}
          />
        </>
      ) : (
        <motion.div className="w-full h-full">
          <motion.img
            src={imageNotAvailable}
            alt="Image Not Available"
            className={`${className} rounded-md`}
            style={{ aspectRatio: '3/4' }}
            initial={{ opacity: 0, scale: 0.9 }}
            animate={{ opacity: 1, scale: 1 }}
            transition={{
              duration: 0.4,
              type: 'spring',
              stiffness: 100,
              damping: 15,
            }}
          />
        </motion.div>
      )}
    </motion.div>
  );
};

export default BookThumbnail;
