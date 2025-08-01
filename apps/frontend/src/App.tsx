import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import type { Book } from '@/api/types';
import { Box, Flex, Heading } from '@radix-ui/themes';
import '@/App.css';
import { LoaderCircle } from 'lucide-react';
import RecommendationList from '@/components/RecommendationList';
import FingerprintJS from '@fingerprintjs/fingerprintjs';
import SearchForm from '@/components/SearchForm';

const App = () => {
  const [recommendations, setRecommendations] = useState<Book[]>([]);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [allRecommendations, setAllRecommendations] = useState<Book[]>([]);
  const [visibleCount, setVisibleCount] = useState<number>(10);
  const [showLoader, setShowLoader] = useState<boolean>(false);
  const [loading, setLoading] = useState<boolean>(false);

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

  const headerVariants = {
    initial: { opacity: 0, y: -30 },
    animate: {
      opacity: 1,
      y: 0,
      transition: {
        duration: 0.8,
        ease: 'easeOut' as const,
        delay: 0.2,
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
          <motion.div
            variants={headerVariants}
            initial="initial"
            animate="animate"
            className="w-full flex justify-center"
          >
            <Box
              minWidth="40%"
              p="4"
              style={{
                border: '1px dashed var(--accent-8)',
                textAlign: 'center',
                borderRadius: '6px',
                boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
                backgroundColor: 'var(--accent-2)',
                color: 'var(--accent-11)',
              }}
            >
              <motion.img
                src="/book-store.png"
                width={35}
                height={35}
                alt="Book Recommendation System"
                initial={{ opacity: 0, rotate: -180 }}
                animate={{ opacity: 1, rotate: 0 }}
                transition={{
                  type: 'spring',
                  duration: 1,
                  stiffness: 100,
                  damping: 10,
                }}
                whileHover={{
                  rotate: 360,
                  scale: 1.1,
                  transition: { duration: 0.6 },
                }}
              />
              <Heading size="8" asChild>
                <motion.h1
                  className="text-3xl font-bold mb-6"
                  initial={{ opacity: 0, y: 20 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{
                    type: 'spring',
                    duration: 1,
                    delay: 0.3,
                  }}
                >
                  Book Recommendation System
                </motion.h1>
              </Heading>
            </Box>
          </motion.div>

          <SearchForm
            loading={loading}
            setLoading={setLoading}
            deviceId={deviceId}
            setVisibleCount={setVisibleCount}
            setRecommendations={setRecommendations}
            setAllRecommendations={setAllRecommendations}
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
                <RecommendationList recommendations={recommendations} />
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
