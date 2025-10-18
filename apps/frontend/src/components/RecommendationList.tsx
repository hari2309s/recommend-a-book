import { Flex, Grid, Text } from '@radix-ui/themes';
import { motion } from 'framer-motion';
import React, { useEffect, useState } from 'react';
import type { Book } from '@/api/types';
import RecommendationCard from '@/components/RecommendationCard';
import { listVariants, listItemVariants } from '@/utils';

type RecommendationListProps = {
  recommendations: Book[];
  searchPerformed: boolean;
};

const RecommendationList: React.FC<RecommendationListProps> = ({
  recommendations,
  searchPerformed,
}: RecommendationListProps) => {
  const [resetAccordions, setResetAccordions] = useState<boolean>(false);

  useEffect(() => {
    if (searchPerformed) {
      setResetAccordions(true);

      const timer = setTimeout(() => setResetAccordions(false), 0);
      return () => clearTimeout(timer);
    }
  }, [searchPerformed]);

  return (
    <Flex asChild p="4" m="4" direction="column" align="center">
      <motion.div variants={listVariants} initial="hidden" animate="visible">
        {recommendations && recommendations.length > 0 ? (
          <Grid columns={{ initial: '1', sm: '2', md: '3' }} gapY="5" gapX="4" mt="170px">
            {recommendations.map((book) => (
              <motion.div key={`${book.title}-${book.author}`} variants={listItemVariants}>
                <RecommendationCard book={book} resetAccordion={resetAccordions} />
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
