import type { Book } from '@/api/types';
import React from 'react';
import { motion } from 'framer-motion';
import RecommendationList from '@/components/RecommendationList';
import { Flex, Spinner, Text } from '@radix-ui/themes';
import { containerVariants, imageVariants, APP_MESSAGES } from '@/utils';
import { IMAGE_ALT_MESSAGES } from '@/utils';

type RecommendationContainerProps = {
  searchPerformed: boolean;
  loading: boolean;
  recommendations: Book[];
  error: string | null;
  paddingTop: string;
};

const RecommendationContainer: React.FC<RecommendationContainerProps> = ({
  searchPerformed,
  loading,
  recommendations,
  error,
  paddingTop,
}) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.5 }}
      className="w-full min-w-[400px]"
      style={{ paddingTop }}
    >
      {recommendations.length > 0 && (
        <RecommendationList recommendations={recommendations} searchPerformed={searchPerformed} />
      )}
      {recommendations.length === 0 && (
        <Flex
          asChild
          p="4"
          style={{
            borderRadius: '5px',
            height: '350px',
            textAlign: 'center',
            color: error ? 'red' : 'green',
          }}
          direction="column"
          align="center"
          justify="center"
        >
          <motion.div initial="initial" animate="animate" variants={containerVariants}>
            {loading ? (
              <motion.div style={{ marginBottom: '10px', color: 'brown' }}>
                <Spinner size="3" loading={loading} className="mb-4" />
              </motion.div>
            ) : (
              <motion.img
                src={error ? '/error.png' : '/info.png'}
                width={40}
                height={40}
                alt={error ? IMAGE_ALT_MESSAGES.ERROR : IMAGE_ALT_MESSAGES.INFO}
                initial="initial"
                animate="animate"
                variants={imageVariants}
                style={{ marginBottom: '10px' }}
              />
            )}
            <Text size={{ initial: '4', md: '5', lg: '5' }} className="text-[var(--accent-12)]">
              {error
                ? error !== ''
                  ? error
                  : APP_MESSAGES.ERROR_LOADING
                : loading
                  ? APP_MESSAGES.LOADING
                  : APP_MESSAGES.INFO}
            </Text>
          </motion.div>
        </Flex>
      )}
    </motion.div>
  );
};

export default RecommendationContainer;
