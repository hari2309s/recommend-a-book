import { useEffect, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { fetchRecommendations } from '@/api';
import type { Book } from '@/api/types';
import { Box, Button, Flex, Heading, TextField } from '@radix-ui/themes';
import './App.css';
import { Search, Loader2 } from 'lucide-react';
import RecommendationList from '@/components/RecommendationList';
import FingerprintJS from '@fingerprintjs/fingerprintjs';

const App = () => {
  const [input, setInput] = useState('');
  const [recommendations, setRecommendations] = useState<Book[]>([]);
  const [loading, setLoading] = useState(false);
  const [deviceId, setDeviceId] = useState<string | null>(null);
  const [allRecommendations, setAllRecommendations] = useState<Book[]>([]);
  const [visibleCount, setVisibleCount] = useState(10);
  const [showLoader, setShowLoader] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    setVisibleCount(10);
    try {
      const data = await fetchRecommendations(input, deviceId!, 51);
      setAllRecommendations(data.recommendations);
      setRecommendations(data.recommendations.slice(0, 10));
    } catch (error) {
      console.error('Error fetching recommendations:', error);
    }
    setLoading(false);
  };

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

  const formVariants = {
    initial: { opacity: 0, scale: 0.95 },
    animate: {
      opacity: 1,
      scale: 1,
      transition: {
        duration: 0.6,
        ease: 'easeOut' as const,
        delay: 0.4,
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
              minWidth="70%"
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
          <motion.div
            variants={formVariants}
            initial="initial"
            animate="animate"
            className="w-full flex justify-center"
          >
            <Flex
              width="100%"
              p="4"
              gap={{ initial: '2', sm: '4' }}
              align="center"
              justify="center"
              direction={{ initial: 'column', sm: 'row' }}
              style={{
                border: '1px dashed var(--accent-8)',
                textAlign: 'center',
                borderRadius: '6px',
                boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
                backgroundColor: 'var(--accent-2)',
                color: 'var(--accent-11)',
              }}
              asChild
            >
              <motion.form
                onSubmit={handleSubmit}
                className="w-full max-w-lg"
                whileHover={{ scale: 1.02 }}
                transition={{ type: 'spring', stiffness: 300, damping: 20 }}
              >
                <Flex
                  gap="4"
                  direction={{ initial: 'column', sm: 'row' }}
                  align="center"
                  justify="center"
                  className="w-full"
                >
                  <motion.div
                    whileFocus={{ scale: 1.02 }}
                    transition={{ type: 'spring', stiffness: 400, damping: 25 }}
                    className="flex-1 min-w-0"
                  >
                    <TextField.Root
                      size="3"
                      placeholder="Enter your book preferences"
                      value={input}
                      onChange={(e) => setInput(e.target.value)}
                      className="w-full"
                    >
                      <TextField.Slot>
                        <Search height="16" width="16" />
                      </TextField.Slot>
                    </TextField.Root>
                  </motion.div>
                  <motion.div whileHover={{ scale: 1.05 }} whileTap={{ scale: 0.95 }}>
                    <Button
                      variant="soft"
                      type="submit"
                      loading={loading}
                      disabled={loading || input.trim() === ''}
                      size="3"
                      className="whitespace-nowrap"
                    >
                      Get Recommendations{' '}
                      {loading && (
                        <motion.div
                          animate={{ rotate: 360 }}
                          transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
                        >
                          <Loader2 size={16} />
                        </motion.div>
                      )}
                    </Button>
                  </motion.div>
                </Flex>
              </motion.form>
            </Flex>
          </motion.div>
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
                    <Loader2 className="animate-spin text-green-600" />
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
