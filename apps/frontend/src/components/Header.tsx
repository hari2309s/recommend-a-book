import { Flex, Heading } from '@radix-ui/themes';
import React from 'react';
import { motion } from 'framer-motion';
import { imageVariants, headerVariants } from '@/utils/animations';

const Header: React.FC = () => {
  return (
    <Flex
      asChild
      p="4"
      style={{
        border: '1px dashed var(--accent-8)',
        textAlign: 'center',
        borderRadius: '6px',
        boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
        backgroundColor: 'var(--accent-2)',
        color: 'var(--accent-11)',
        width: 'w-[50%]',
      }}
      direction="column"
      justify="center"
      align="center"
    >
      <motion.div
        variants={headerVariants}
        initial="initial"
        animate="animate"
        className="flex justify-center"
      >
        <motion.img
          src="/book-store.png"
          width={35}
          height={35}
          alt="Book Recommendation System"
          initial="initial"
          animate="animate"
          variants={imageVariants}
          whileHover={{
            scale: 1.1,
            transition: { duration: 0.2, ease: 'easeOut' as const },
          }}
        />
        <Heading asChild className="text-sm font-small mb-6">
          <motion.h2
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{
              type: 'spring',
              duration: 1,
              delay: 0.3,
            }}
          >
            Book Recommendation System
          </motion.h2>
        </Heading>
      </motion.div>
    </Flex>
  );
};

export default Header;
