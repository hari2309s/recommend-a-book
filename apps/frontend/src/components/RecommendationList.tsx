import type { FC } from 'react';
import { Flex, Grid, Text } from '@radix-ui/themes';
import { motion } from 'framer-motion';
import type { Book } from '@/api/types';
import { Heading } from '@radix-ui/themes';
import RecommendationCard from '@/components/RecommendationCard';

type RecommendationListProps = {
  recommendations: Book[];
};

const RecommendationList: FC<RecommendationListProps> = ({
  recommendations,
}: RecommendationListProps) => {
  const containerVariants = {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        staggerChildren: 0.1,
        delayChildren: 0.2,
      },
    },
  };

  const itemVariants = {
    hidden: {
      opacity: 0,
      y: 20,
      scale: 0.95,
    },
    visible: {
      opacity: 1,
      y: 0,
      scale: 1,
      transition: {
        type: 'spring' as const,
        stiffness: 100,
        damping: 12,
      },
    },
  };

  return (
    <Flex
      asChild
      width="100%"
      p="4"
      m="4"
      direction="column"
      align="center"
      style={{ marginTop: '100px' }}
    >
      <motion.div variants={containerVariants} initial="hidden" animate="visible">
        <motion.div
          initial={{ opacity: 0, y: -20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.6, ease: 'easeOut' }}
        >
          <Heading size="6" mb="6" className="text-center" color="green">
            Recommended Books
          </Heading>
        </motion.div>
        {recommendations && recommendations.length > 0 ? (
          <Grid columns={{ initial: '1', sm: '2', md: '3' }} gapY="5" gapX="4">
            {recommendations.map((book, index) => (
              <motion.div key={index} variants={itemVariants}>
                <RecommendationCard book={book} />
              </motion.div>
            ))}
          </Grid>
        ) : (
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 0.5 }}
          >
            <Text>No recommendations found. Search again.</Text>
          </motion.div>
        )}
      </motion.div>
    </Flex>
  );
};

export default RecommendationList;
