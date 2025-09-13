import { Box } from '@radix-ui/themes';
import { motion } from 'framer-motion';
import { Text } from '@radix-ui/themes';
import React from 'react';

const Empty: React.FC = () => {
  return (
    <Box
      asChild
      width="full"
      p="4"
      style={{
        marginTop: '180px',
        border: '1px dashed green',
        borderRadius: '5px',
        width: '100%',
        height: '350px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
      }}
    >
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.5, ease: 'easeInOut' }}
        className="flex flex-col"
      >
        <motion.img
          src="/info.png"
          width={40}
          height={40}
          alt="Error"
          initial={{ opacity: 0, rotate: -180 }}
          animate={{ opacity: 1, rotate: 0 }}
          transition={{
            type: 'spring',
            duration: 1,
            stiffness: 100,
            damping: 10,
          }}
        />
        {<Text size="5">Try searching for a book</Text>}
      </motion.div>
    </Box>
  );
};

export default Empty;
