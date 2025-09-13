import { Box } from '@radix-ui/themes';
import { motion } from 'framer-motion';
import { Text } from '@radix-ui/themes';
import React from 'react';

type ErrorProps = {
  message?: string;
};

const Error: React.FC<ErrorProps> = ({ message }: ErrorProps) => {
  return (
    <Box
      asChild
      width="full"
      p="4"
      style={{
        marginTop: '180px',
        border: '1px dashed red',
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
          src="/error.png"
          width={35}
          height={35}
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
        {message ? (
          <Text size="5" color="red">
            {message}
          </Text>
        ) : (
          <Text size="5" color="red">
            An error occurred. Try again later.
          </Text>
        )}
      </motion.div>
    </Box>
  );
};

export default Error;
