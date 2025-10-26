import { Flex, Heading } from '@radix-ui/themes';
import React from 'react';
import { motion } from 'framer-motion';
import { imageVariants, headerVariants } from '@/utils';

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
        color: 'brown',
      }}
      direction="column"
      justify="center"
      align="center"
      gap="4"
    >
      <motion.div variants={headerVariants} initial="initial" animate="animate">
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
        <Heading size={{ initial: '3', sm: '4', md: '5', lg: '6' }}>
          <a
            href="/"
            onClick={() => window.location.reload()}
            style={{ all: 'unset', cursor: 'pointer' }}
          >
            Book Recommendation System
          </a>
        </Heading>
      </motion.div>
    </Flex>
  );
};

export default Header;
