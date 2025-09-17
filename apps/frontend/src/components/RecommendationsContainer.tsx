import type { Book } from '@/api/types';
import React from 'react';
import { motion } from 'framer-motion';
import RecommendationList from '@/components/RecommendationList';
import { Flex, Spinner, Text } from '@radix-ui/themes';
import { containerVariants, imageVariants } from '@/utils/animations';
import { RECOMMENDATIONS_MESSAGES } from '@/utils/messages';

type RecommendationContainerProps = {
  searchPerformed: boolean;
  loading: boolean;
  recommendations: Book[];
  error: string | null;
};

const RecommendationContainer: React.FC<RecommendationContainerProps> = ({
  searchPerformed,
  loading,
  recommendations,
  error,
}) => {
  return (
    <motion.div
      initial={{ opacity: 0, y: 20 }}
      animate={{ opacity: 1, y: 0 }}
      exit={{ opacity: 0, y: -20 }}
      transition={{ duration: 0.5 }}
      className="w-full min-w-[350px]"
    >
      {recommendations.length > 0 && (
        <RecommendationList recommendations={recommendations} searchPerformed={searchPerformed} />
      )}
      {recommendations.length === 0 && (
        <Flex
          asChild
          p="4"
          style={{
            marginTop: '180px',
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
              <motion.div style={{ marginBottom: '10px' }}>
                <Spinner size="3" loading={loading} className="mb-4" />
              </motion.div>
            ) : (
              <motion.img
                src={error ? '/error.png' : '/info.png'}
                width={40}
                height={40}
                alt="Error"
                initial="initial"
                animate="animate"
                variants={imageVariants}
                style={{ marginBottom: '10px' }}
              />
            )}
            <Text size="5" className="text-[var(--accent-12)]">
              {error
                ? error !== ''
                  ? error
                  : RECOMMENDATIONS_MESSAGES.ERROR_LOADING
                : loading
                  ? RECOMMENDATIONS_MESSAGES.LOADING
                  : RECOMMENDATIONS_MESSAGES.INFO}
            </Text>
          </motion.div>
        </Flex>
      )}
    </motion.div>
  );
};

export default RecommendationContainer;
