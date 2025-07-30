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
  return (
    <Flex asChild width="100%" p="4" m="4" direction="column" align="center">
      <motion.div>
        <Heading size="6" mb="6" className="text-center" color="green">
          Recommended Books
        </Heading>
        {recommendations && recommendations.length > 0 ? (
          <Grid columns={{ initial: '1', sm: '2', md: '3' }} gapY="5" gapX="4">
            {recommendations.map((book, index) => (
              <motion.div key={index}>
                <RecommendationCard book={book} />
              </motion.div>
            ))}
          </Grid>
        ) : (
          <Text>No recommendations found. Search again.</Text>
        )}
      </motion.div>
    </Flex>
  );
};

export default RecommendationList;
