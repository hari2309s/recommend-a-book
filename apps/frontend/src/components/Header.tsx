import { Box, Heading } from '@radix-ui/themes';
import type { FC } from 'react';
import { motion } from 'framer-motion';

const Header: FC = () => {
  const headerVariants = {
    initial: { opacity: 0, y: -30 },
    animate: {
      opacity: 1,
      y: 0,
      transition: {
        duration: 0.8,
        ease: 'easeOut' as const,
        delay: 0.2,
      },
    },
  };

  return (
    <motion.div
      variants={headerVariants}
      initial="initial"
      animate="animate"
      className="w-full flex justify-center"
    >
      <Box
        minWidth="40%"
        p="4"
        style={{
          border: '1px dashed var(--accent-8)',
          textAlign: 'center',
          borderRadius: '6px',
          boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
          backgroundColor: 'var(--accent-2)',
          color: 'var(--accent-11)',
        }}
      >
        <motion.img
          src="/book-store.png"
          width={35}
          height={35}
          alt="Book Recommendation System"
          initial={{ opacity: 0, rotate: -180 }}
          animate={{ opacity: 1, rotate: 0 }}
          transition={{
            type: 'spring',
            duration: 1,
            stiffness: 100,
            damping: 10,
          }}
          whileHover={{
            rotate: 360,
            scale: 1.1,
            transition: { duration: 0.6 },
          }}
        />
        <Heading size="7" asChild>
          <motion.h1
            className="text-3xl font-bold mb-6"
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{
              type: 'spring',
              duration: 1,
              delay: 0.3,
            }}
          >
            Book Recommendation System
          </motion.h1>
        </Heading>
      </Box>
    </motion.div>
  );
};

export default Header;
