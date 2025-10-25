import React, { useState } from 'react';
import { motion, useScroll, useMotionValueEvent } from 'framer-motion';
import { Button, Flex, TextField, Badge, Text } from '@radix-ui/themes';
import { Search, Tag } from 'lucide-react';
import { toast } from 'sonner';
import type { Book } from '@/api/types';
import { fetchRecommendations } from '@/api';
import { containerVariants } from '@/utils';
import { coldStartToastManager } from '@/utils/coldStartToast';

type SearchFormProps = {
  loading: boolean;
  setLoading: (loading: boolean) => void;
  resetScroll: () => void;
  setAllRecommendations: (books: Book[]) => void;
  setErrorMessage: (message: string | null) => void;
};

const SearchForm: React.FC<SearchFormProps> = ({
  loading,
  setLoading,
  resetScroll,
  setAllRecommendations,
  setErrorMessage,
}: SearchFormProps) => {
  const [isSticky, setIsSticky] = useState<boolean>(false);
  const { scrollY } = useScroll();
  const [input, setInput] = useState<string>('');
  const [currentSemanticTags, setCurrentSemanticTags] = useState<string[]>([]);

  useMotionValueEvent(scrollY, 'change', (latest: number) => {
    setIsSticky(latest > 140);
  });

  const handleSubmit = async (e: React.FormEvent<HTMLFormElement>): Promise<void> => {
    e.preventDefault();
    if (!input.trim()) return;

    setLoading(true);
    setErrorMessage(null);
    setCurrentSemanticTags([]);
    resetScroll();

    // Start cold start toast timer
    coldStartToastManager.start();

    try {
      const data = await fetchRecommendations(input, 100, {
        onRetry: (attempt, maxRetries) => {
          coldStartToastManager.retry(attempt, maxRetries);
        },
      });

      // Dismiss toast immediately when results arrive
      coldStartToastManager.dismiss();

      if (data.recommendations && Array.isArray(data.recommendations)) {
        setAllRecommendations(data.recommendations);
        setCurrentSemanticTags(data.semantic_tags || []);
      } else {
        console.error('Invalid recommendations format received');
        toast.error('Invalid response format', {
          description: 'Please try again.',
        });
      }
    } catch (error) {
      console.error('Error fetching recommendations:', error);

      // Dismiss toast on error
      coldStartToastManager.dismiss();

      let errorMessage = 'Failed to fetch recommendations';
      let errorDescription = 'Please try again later.';

      if (error instanceof Error) {
        if (error.message.includes('timed out')) {
          errorMessage = 'Request timed out';
          errorDescription = 'The API is taking longer than expected. Please try again.';
        } else if (error.message.includes('Network error')) {
          errorMessage = 'Connection failed';
          errorDescription = 'Could not reach the API server. Check your connection.';
        } else {
          errorDescription = error.message;
        }
      }

      setErrorMessage(errorMessage);
      toast.error(errorMessage, {
        description: errorDescription,
        duration: 5000,
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <motion.div
      className="z-50 fixed left-4 right-4 w-[80%] sm:w-[50%] bg-secondary/40 backdrop-blur-lg
      border border-dashed border-primary/20 shadow-lg shadow-primary/10 flex flex-col gap-[10px]
      justify-center items-center min-h-[75px]"
      style={{
        padding: '19px',
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        background: 'rgba(245, 245, 220, 0.4)',
        borderRadius: '6px',
        borderColor: '#8B4513',
      }}
      variants={{
        initial: {
          opacity: 0,
          scale: 0.95,
        },
        animate: {
          opacity: 1,
          scale: 1,
          transition: {
            duration: 0.6,
            ease: 'easeOut',
            delay: 0.4,
          },
        },
      }}
      initial="initial"
      animate={{
        ...containerVariants.animate,
        ...{
          opacity: 1,
          scale: 1,
        },
        top: isSticky ? 50 : 190,
        boxShadow: isSticky
          ? '0 25px 50px -12px rgba(139, 69, 19, 0.25), 0 8px 16px -8px rgba(139, 69, 19, 0.1)'
          : '0 10px 25px -5px rgba(139, 69, 19, 0.1), 0 4px 6px -2px rgba(139, 69, 19, 0.05)',
      }}
      transition={{
        opacity: { duration: 0.6, ease: 'easeOut', delay: 0.4 },
        scale: { duration: 0.6, ease: 'easeOut', delay: 0.4 },
        top: {
          type: 'spring',
          stiffness: 300,
          damping: 25,
          mass: 0.7,
        },
        boxShadow: {
          duration: 0.2,
          ease: 'easeInOut',
        },
      }}
      whileHover={{
        scale: 1.01,
        transition: {
          type: 'spring',
          stiffness: 400,
          damping: 25,
        },
      }}
    >
      <motion.form onSubmit={handleSubmit} className="w-full">
        <Flex
          gap="4"
          direction={{ initial: 'column', sm: 'row' }}
          align={{ initial: 'center' }}
          justify="center"
          className="w-full contents"
        >
          <motion.div
            whileFocus={{
              scale: 1.02,
              transition: { type: 'spring', stiffness: 400, damping: 25 },
            }}
            className="flex-1 w-full"
          >
            <TextField.Root
              id="search-input"
              size="3"
              placeholder="Enter your book preferences"
              value={input}
              onChange={(e: React.ChangeEvent<HTMLInputElement>) => setInput(e.target.value)}
              onBlur={(e: React.FocusEvent<HTMLInputElement>) => setInput(e.target.value)}
              className="w-full"
              style={{
                backgroundColor: 'rgba(255, 255, 255, 0.9)',
                border: '1px solid rgba(139, 69, 19, 0.3)',
                outline: '1px solid rgba(139, 69, 19, 0.5)',
                outlineOffset: '0px',
              }}
            >
              <TextField.Slot>
                <Search height="16" width="16" />
              </TextField.Slot>
            </TextField.Root>
          </motion.div>
          <motion.div
            whileHover={{
              scale: 1.05,
              transition: { type: 'spring', stiffness: 400, damping: 20 },
            }}
            whileTap={{
              scale: 0.95,
              transition: { type: 'spring', stiffness: 400, damping: 20 },
            }}
          >
            <Button
              variant="soft"
              type="submit"
              loading={loading}
              disabled={loading || !input.trim()}
              size="3"
              className="whitespace-nowrap bg-primary hover:bg-primary-dark text-white"
              style={{
                backgroundColor: '#8B4513',
                color: 'white',
              }}
            >
              Get Recommendations
            </Button>
          </motion.div>
        </Flex>
      </motion.form>

      {currentSemanticTags.length > 0 && (
        <motion.div
          initial={{ opacity: 0, y: 20 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{ duration: 0.5, delay: 0.3 }}
          className="w-full max-w-4xl mt-4 m-3"
        >
          <Flex gap="2" direction="column" align="center">
            <Flex gap="2" align="center">
              <Tag size={16} className="text-primary" />
              <Text size="2" className="text-primary-dark font-medium">
                Detected themes:
              </Text>
            </Flex>
            <Flex gap="2" wrap="wrap" justify="center">
              {currentSemanticTags.map((tag: string, index: number) => (
                <motion.div
                  key={tag}
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ duration: 0.3, delay: 0.4 + index * 0.1 }}
                >
                  <Badge
                    variant="solid"
                    size="3"
                    className="text-sm bg-primary/20 text-primary-dark"
                  >
                    {tag}
                  </Badge>
                </motion.div>
              ))}
            </Flex>
          </Flex>
        </motion.div>
      )}
    </motion.div>
  );
};

export default SearchForm;
