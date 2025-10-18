import React, { useState } from 'react';
import { motion, useScroll, useMotionValueEvent } from 'framer-motion';
import { Button, Flex, TextField, Badge, Text } from '@radix-ui/themes';
import { Search, Tag } from 'lucide-react';
import { toast } from 'sonner';
import type { Book, ColdStartInfo } from '@/api/types';
import { fetchRecommendations } from '@/api';
import { containerVariants } from '@/utils';
import { usePrewarm } from '@/hooks';

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
  const [coldStartToastId, setColdStartToastId] = useState<string | number | null>(null);
  const [currentSemanticTags, setCurrentSemanticTags] = useState<string[]>([]);

  // Use the prewarming hook
  const { isPrewarmed, prewarmApi, isPrewarming } = usePrewarm();

  useMotionValueEvent(scrollY, 'change', (latest) => {
    setIsSticky(latest > 140);
  });

  const handleColdStart = (info: ColdStartInfo) => {
    // Dismiss any existing cold start toast
    if (coldStartToastId) {
      toast.dismiss(coldStartToastId);
    }

    let message = 'API is warming up...';
    let description = 'This may take a moment on the first request. Retrying automatically.';

    switch (info.reason) {
      case 'first_request':
        message = '🔥 Warming up the API...';
        description =
          'First request detected. The API is starting up. This will be faster next time!';
        break;
      case 'timeout':
        message = '⏱️ Request timed out';
        description = 'The API is experiencing a cold start. Retrying with extended timeout...';
        break;
      case 'slow_response':
        message = '🐌 Slow response detected';
        description = 'The API might be cold starting. Hang tight, retrying...';
        break;
      case 'network_error':
        message = '🌐 Connection issue';
        description = 'Attempting to reconnect to the API...';
        break;
    }

    const toastId = toast.loading(message, {
      description,
      duration: Infinity, // Keep it visible until we dismiss it
    });

    setColdStartToastId(toastId);
  };

  const handleRetry = (attempt: number, maxRetries: number) => {
    if (coldStartToastId) {
      toast.loading(`Retry ${attempt}/${maxRetries}...`, {
        id: coldStartToastId,
        description: `Still warming up. Please wait...`,
      });
    }
  };

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!input.trim()) return;

    setLoading(true);
    setErrorMessage(null);
    resetScroll();
    setColdStartToastId(null);

    try {
      // If API is not prewarmed, try to prewarm it first
      if (!isPrewarmed && !isPrewarming) {
        toast.loading('Preparing API...', {
          description: 'Ensuring the API is ready for your request.',
          duration: 2000,
        });

        try {
          await prewarmApi(true);
          toast.dismiss();
        } catch (prewarmError) {
          console.warn('Prewarm failed, proceeding with request:', prewarmError);
          toast.dismiss();
        }
      }

      const data = await fetchRecommendations(input, 100, {
        onColdStart: handleColdStart,
        onRetry: handleRetry,
      });

      // Dismiss cold start toast on success
      if (coldStartToastId) {
        toast.dismiss(coldStartToastId);
        toast.success('Recommendations loaded!', {
          description: `Found ${data.recommendations.length} books for you.`,
          duration: 3000,
        });
        setColdStartToastId(null);
      }

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

      // Dismiss cold start toast on error
      if (coldStartToastId) {
        toast.dismiss(coldStartToastId);
        setColdStartToastId(null);
      }

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
      className="z-50 fixed left-4 right-4 w-[80%] sm:w-[50%] bg-green-500/40
        backdrop-blur-lg border border-dashed border-green-300/20 shadow-lg
        shadow-green-500/10 flex flex-col gap-[10px] justify-center items-center min-h-[75px]"
      style={{
        padding: '19px',
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        background: 'rgba(34, 197, 94, 0.4)',
        borderRadius: '6px',
        borderColor: 'green',
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
          ? '0 25px 50px -12px rgba(34, 197, 94, 0.25), 0 8px 16px -8px rgba(34, 197, 94, 0.1)'
          : '0 10px 25px -5px rgba(34, 197, 94, 0.1), 0 4px 6px -2px rgba(34, 197, 94, 0.05)',
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
              className="whitespace-nowrap bg-green-600 hover:bg-green-700 text-white"
              style={{
                backgroundColor: 'rgb(34, 197, 94)',
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
              <Tag size={16} className="text-green-600" />
              <Text size="2" className="text-green-700 font-medium">
                Detected themes:
              </Text>
            </Flex>
            <Flex gap="2" wrap="wrap" justify="center">
              {currentSemanticTags.map((tag, index) => (
                <motion.div
                  key={tag}
                  initial={{ opacity: 0, scale: 0.8 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ duration: 0.3, delay: 0.4 + index * 0.1 }}
                >
                  <Badge variant="solid" size="3" className="text-sm">
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
