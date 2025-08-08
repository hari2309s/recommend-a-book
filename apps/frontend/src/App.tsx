import { useEffect, useState, type FC } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { Book } from '@/api/types';
import { Box, Flex } from '@radix-ui/themes';
import '@/App.css';
import { LoaderCircle } from 'lucide-react';
import { RecommendationList } from '@/components/RecommendationList';
import FingerprintJS from '@fingerprintjs/fingerprintjs';
import SearchForm from '@/components/SearchForm';
import Header from '@/components/Header';

const App: FC = () => {
  const [recommendations, setRecommendations] = useState<Book[]>([]);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [allRecommendations, setAllRecommendations] = useState<Book[]>([]);
  const [visibleCount, setVisibleCount] = useState<number>(10);
  const [showLoader, setShowLoader] = useState<boolean>(false);
  const [loading, setLoading] = useState<boolean>(false);
  const [searchPerformed, _] = useState<boolean>(false);

  useEffect(() => {
    const handleScroll = () => {
      if (
        window.innerHeight + window.scrollY >= document.body.offsetHeight - 100 &&
        !loading &&
        visibleCount < allRecommendations.length
      ) {
        setShowLoader(true);
        setTimeout(() => {
          setVisibleCount((prev) => Math.min(prev + 10, allRecommendations.length));
          setShowLoader(false);
        }, 600);
      }
    };
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, [loading, visibleCount, allRecommendations.length]);

  useEffect(() => {
    setRecommendations(allRecommendations.slice(0, visibleCount));
  }, [visibleCount, allRecommendations]);

  useEffect(() => {
    const initializeFingerprint = async () => {
      const fp = await FingerprintJS.load();
      const result = await fp.get();
      setDeviceId(result.visitorId);
    };
    initializeFingerprint();
  }, []);

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
            deviceId={deviceId}
            setVisibleCount={setVisibleCount}
            setAllRecommendations={setAllRecommendations}
            setRecommendations={setRecommendations}
          />
          <AnimatePresence mode="wait">
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
