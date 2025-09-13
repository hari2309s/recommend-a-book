import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { Book } from '@/api/types';
import { Box, Flex } from '@radix-ui/themes';
import '@/App.css';
import { LoaderCircle } from 'lucide-react';
import RecommendationList from '@/components/RecommendationList';
import SearchForm from '@/components/SearchForm';
import Header from '@/components/Header';
import { useInfiniteScroll } from '@/hooks';
import Error from '@/components/Error';
import Empty from './components/Empty';

const App: React.FC = () => {
  const [loading, setLoading] = useState<boolean>(false);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  const {
    visibleItems: recommendations,
    setAllItems: setAllRecommendations,
    isLoading: showLoader,
    resetScroll,
    searchPerformed,
  } = useInfiniteScroll<Book>({
    initialItemsToShow: 10,
    itemsToLoadPerPage: 10,
    threshold: 100,
  });

  const pageVariants = {
    initial: { opacity: 0, y: 20 },
    animate: {
      opacity: 1,
      y: 0,
      transition: {
        duration: 0.6,
        ease: 'easeOut' as const,
      },
    },
  };

  return (
    <motion.div
      variants={pageVariants}
      initial="initial"
      animate="animate"
      className="min-h-screen"
    >
      <Box
        minHeight="100vh"
        minWidth="100vw"
        p="8"
        style={{ backgroundColor: 'var(--accent-1)' }}
        className="max-w-screen"
      >
        <Flex direction="column" gap="4" align="center">
          <Header />
          <SearchForm
            loading={loading}
            setLoading={setLoading}
            resetScroll={resetScroll}
            setAllRecommendations={setAllRecommendations}
            setErrorMessage={setErrorMessage}
          />
          <AnimatePresence mode="wait">
            {errorMessage && <Error message={errorMessage} />}
            {errorMessage === null && recommendations.length === 0 && <Empty />}
            {recommendations?.length > 0 && (
              <motion.div
                key="recommendations"
                initial={{ opacity: 0, y: 20 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: -20 }}
                transition={{ duration: 0.5 }}
              >
                <RecommendationList
                  recommendations={recommendations}
                  searchPerformed={searchPerformed}
                />
                {showLoader && (
                  <div
                    style={{
                      height: 40,
                      display: 'flex',
                      alignItems: 'center',
                      justifyContent: 'center',
                    }}
                  >
                    <LoaderCircle className="animate-spin text-green-600" />
                  </div>
                )}
              </motion.div>
            )}
          </AnimatePresence>
        </Flex>
      </Box>
    </motion.div>
  );
};

export default App;
