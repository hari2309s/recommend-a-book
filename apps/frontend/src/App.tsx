import React, { useState } from 'react';
import { motion } from 'framer-motion';
import type { Book } from '@/api/types';
import { Flex } from '@radix-ui/themes';
import '@/App.css';
import SearchForm from '@/components/SearchForm';
import Header from '@/components/Header';
import { useInfiniteScroll } from '@/hooks';
import { pageVariants } from '@/utils/animations';
import RecommendationContainer from '@/components/RecommendationContainer';

const App: React.FC = () => {
  const [loading, setLoading] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const {
    visibleItems: recommendations,
    setAllItems: setAllRecommendations,
    isLoading: isScrollLoading,
    resetScroll,
    searchPerformed,
  } = useInfiniteScroll<Book>({
    initialItemsToShow: 10,
    itemsToLoadPerPage: 10,
    threshold: 100,
  });

  return (
    <Flex
      asChild
      minHeight="100vh"
      minWidth="100vw"
      p="8"
      style={{ backgroundColor: 'var(--accent-1)' }}
      className="max-w-screen"
      direction="column"
      align="center"
      justify="center"
      gap="4"
    >
      <motion.div
        variants={pageVariants}
        initial="initial"
        animate="animate"
        className="min-h-screen"
      >
        <Header />
        <SearchForm
          loading={loading}
          setLoading={setLoading}
          resetScroll={resetScroll}
          setAllRecommendations={setAllRecommendations}
          setErrorMessage={setErrorMessage}
        />

        <RecommendationContainer
          searchPerformed={searchPerformed}
          loading={loading || isScrollLoading}
          recommendations={recommendations}
          error={errorMessage}
        />
      </motion.div>
    </Flex>
  );
};

export default App;
