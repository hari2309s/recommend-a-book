import { Box } from '@radix-ui/themes';
import { motion } from 'framer-motion';
import { Text } from '@radix-ui/themes';

type ErrorProps = {
  message?: string;
};

const Error = ({ message }: ErrorProps) => {
  return (
    <Box
      asChild
      width="full"
      p="4"
      style={{
        marginTop: '150px',
        border: '1px dashed red',
        borderRadius: '5px',
        width: '70%',
        height: '300px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        background: 'rgba(255, 0, 0, 0.1)',
      }}
    >
      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.5, ease: 'easeInOut' }}
      >
        {message ? (
          <Text size="5">{message}</Text>
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
