import React, { useState } from 'react';
import { motion, useScroll } from 'framer-motion';
import { Button, Flex, TextField } from '@radix-ui/themes';
import { LoaderCircle, Search } from 'lucide-react';
import type { Book } from '@/api/types';
import { fetchRecommendations } from '@/api';

type SearchFormProps = {
  loading: boolean;
  setLoading: (loading: boolean) => void;
  deviceId: string | null;
  resetScroll: () => void;
  setAllRecommendations: (books: Book[]) => void;
  setErrorMessage: (message: string | null) => void;
};

const SearchForm: React.FC<SearchFormProps> = ({
  loading,
  setLoading,
  deviceId,
  resetScroll,
  setAllRecommendations,
  setErrorMessage,
}: SearchFormProps) => {
  const [isSticky, setIsSticky] = useState<boolean>(false);
  const { scrollY } = useScroll();
  const [input, setInput] = useState<string>('');

  scrollY.on('change', (latest) => {
    if (latest > 50) {
      setIsSticky(true);
    } else {
      setIsSticky(false);
    }
  });

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    setLoading(true);
    setErrorMessage(null);
    resetScroll();

    try {
      const data = await fetchRecommendations(input, deviceId || '', 50);
      if (data.recommendations && Array.isArray(data.recommendations)) {
        setAllRecommendations(data.recommendations);
      } else {
        console.error('Invalid recommendations format received');
      }
    } catch (error) {
      console.error('Error fetching recommendations:', error);
      setErrorMessage('Failed to fetch recommendations');
    } finally {
      setLoading(false);
    }
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
      className={`z-50 fixed top-[190px] left-4 right-4 w-[70%] sm:w-[50%] bg-green-500/40
        backdrop-blur-lg border border-dashed border-green-300/20 shadow-lg
        shadow-green-500/10 flex justify-center items-center min-h-[75px]
        ${isSticky ? 'shadow-2xl shadow-green-500/20' : 'shadow-lg shadow-green-500/10'}`}
      style={{
        padding: '0 19px',
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        background: 'rgba(34, 197, 94, 0.4)',
        borderRadius: '6px',
        borderColor: 'green',
        boxShadow: isSticky
          ? '0 25px 50px -12px rgba(34, 197, 94, 0.25), 0 8px 16px -8px rgba(34, 197, 94, 0.1)'
          : '0 10px 25px -5px rgba(34, 197, 94, 0.1), 0 4px 6px -2px rgba(34, 197, 94, 0.05)',
      }}
      whileHover={{
        scale: 1.01,
      }}
      transition={{ type: 'spring', stiffness: 300, damping: 20 }}
      variants={formVariants}
      initial="initial"
      animate="animate"
    >
      <motion.form onSubmit={handleSubmit} className="w-full">
        <Flex gap="4" direction={{ sm: 'row' }} align="center" justify="center" className="w-full">
          <motion.div
            whileFocus={{ scale: 1.02 }}
            transition={{ type: 'spring', stiffness: 400, damping: 25 }}
            className="flex-1 max-w-md"
          >
            <TextField.Root
              size="3"
              placeholder="Enter your book preferences"
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onBlur={(e) => setInput(e.target.value)}
              className="w-full"
              style={{
                backgroundColor: 'rgba(255, 255, 255, 0.9)',
                border: '1px solid rgba(34, 197, 94, 0.3)',
                outline: '1px solid rgba(34, 197, 94, 0.5)',
                outlineOffset: '0px',
              }}
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
              disabled={loading || !input.trim()}
              size="3"
              className="whitespace-nowrap bg-green-600 hover:bg-green-700 text-white"
              style={{
                backgroundColor: 'rgb(34, 197, 94)',
                color: 'white',
              }}
            >
              Get Recommendations{' '}
              {loading && (
                <motion.div
                  animate={{ rotate: 360 }}
                  transition={{ duration: 1, repeat: Infinity, ease: 'linear' }}
                >
                  <LoaderCircle size={16} />
                </motion.div>
              )}
            </Button>
          </motion.div>
        </Flex>
      </motion.form>
    </motion.div>
  );
};

export default SearchForm;
