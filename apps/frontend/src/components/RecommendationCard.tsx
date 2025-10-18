import type { Book } from '@/api/types';
import { Card, Heading, Badge, Flex, Text, Separator } from '@radix-ui/themes';
import { motion } from 'framer-motion';
import { ShoppingCartIcon, Star, Target, TrendingUp } from 'lucide-react';
import React, { useState, useEffect } from 'react';
import BookThumbnail from '@/components/BookThumbnail';
import AuthorBadges from '@/components/AuthorBadges';
import BookDescriptionAccordion from '@/components/BookDescriptionAccordion';
import { formatRatingsCount, getBookStoreLinks, cardVariants } from '@/utils';

type RecommendationCardProps = {
  book: Book;
  resetAccordion: boolean;
};

const RecommendationCard: React.FC<RecommendationCardProps> = ({ book, resetAccordion }) => {
  const [isAccordionOpen, setIsAccordionOpen] = useState<boolean>(false);

  useEffect(() => {
    if (resetAccordion) {
      setIsAccordionOpen(false);
    }
  }, [resetAccordion]);

  return (
    <Card
      asChild
      size={{ initial: '1', sm: '2', md: '3' }}
      style={{
        textAlign: 'right',
        backgroundColor: 'var(--accent-6)',
        color: 'var(--accent-11)',
        minHeight: '290px',
        cursor: 'pointer',
        transition: 'box-shadow 0.3s ease',
      }}
      className="hover:shadow-lg hover:shadow-green-200/50 border-0"
    >
      <motion.div
        whileHover={{
          y: -8,
          scale: 1.02,
          transition: { type: 'spring', stiffness: 300, damping: 20 },
        }}
        whileTap={{ scale: 0.98 }}
        initial="initial"
        animate="animate"
        variants={cardVariants}
      >
        <Flex gap="2" direction="column" align="stretch">
          <Flex gap="2" direction="row" justify="between">
            <BookThumbnail src={book.thumbnail} alt={book.title || 'Book cover'} className="mr-4" />
            <Flex
              gap="2"
              direction="column"
              align="end"
              className="max-h-48! text-ellipsis overflow-hidden"
            >
              <motion.div
                initial={{ opacity: 0, x: 20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ duration: 0.4, delay: 0.2 }}
              >
                <Heading size={{ initial: '6' }} className="text-ellipsis overflow-hidden">
                  {book.title}
                </Heading>
              </motion.div>
              {book.author && <AuthorBadges authors={book.author.split(',')} />}
            </Flex>
          </Flex>
          <Separator size="4" />
          <Flex direction="column" justify="center" align="end" gap="2" className="align-normal">
            <motion.div
              initial={{ opacity: 0, scale: 0.8 }}
              animate={{ opacity: 1, scale: 1 }}
              transition={{ duration: 0.3, delay: 0.3 }}
              className="w-full flex justify-between"
            >
              <Badge size="3" variant="surface" className="max-w-max">
                {book.categories
                  ? book.categories.reduce(
                      (acc, curr) => (curr && acc ? `${curr} | ${acc}` : curr),
                      ''
                    )
                  : 'Unknown'}
              </Badge>
              <div>
                {getBookStoreLinks(book.isbn).map((store, index) => (
                  <motion.a
                    key={index}
                    href={store.url}
                    title={`Buy from ${store.name}`}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="inline-block"
                    whileHover={{
                      x: index === 0 ? -8 : 8,
                    }}
                    whileTap={{
                      scale: 0.95,
                    }}
                    transition={{
                      type: 'spring',
                      stiffness: 400,
                      damping: 10,
                    }}
                  >
                    <ShoppingCartIcon fill="green" color="green" />
                  </motion.a>
                ))}
              </div>
            </motion.div>
            <Flex gap="3" justify="between" align="center" className="w-full">
              <Flex gap="3" justify="center" align="center">
                <motion.div
                  initial={{ rotate: -180, opacity: 0 }}
                  animate={{ rotate: 0, opacity: 1 }}
                  transition={{ duration: 0.5, delay: 0.4 }}
                >
                  <Star fill="green" />
                </motion.div>
                <Text>
                  {book.rating?.toFixed(2)} / {formatRatingsCount(book.ratings_count || 0)}
                </Text>
              </Flex>
              {book.published_year ||
                (book.year && (
                  <motion.div
                    initial={{ opacity: 0 }}
                    animate={{ opacity: 1 }}
                    transition={{ duration: 0.3, delay: 0.5 }}
                  >
                    <Text className="italic">{book.published_year || book.year}</Text>
                  </motion.div>
                ))}
            </Flex>

            {/* Confidence Score */}
            <Flex gap="2" align="center" direction="row" className="w-full">
              <motion.div
                initial={{ opacity: 0, x: -20 }}
                animate={{ opacity: 1, x: 0 }}
                transition={{ duration: 0.4, delay: 0.6 }}
              >
                <TrendingUp size={16} className="text-green-600" />
              </motion.div>
              <Text size="2" className="text-green-700 font-medium">
                {Math.round(book.confidence_score * 100)}% match
              </Text>
            </Flex>

            {/* Relevance Indicators */}
            {book.relevance_indicators && book.relevance_indicators.length > 0 && (
              <Flex gap="2" align="center" className="w-full">
                <motion.div
                  initial={{ opacity: 0, y: 10 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ duration: 0.4, delay: 0.7 }}
                >
                  <Flex gap="1" align="center">
                    <Target size={19} className="text-green-600" />
                    <Text size="2" className="text-green-700 font-medium">
                      Matches your interest in:
                    </Text>
                  </Flex>
                </motion.div>
                <Flex gap="1" wrap="wrap" className="ml-5">
                  {book.relevance_indicators.slice(0, 3).map((indicator, index) => (
                    <motion.div
                      key={indicator}
                      initial={{ opacity: 0, scale: 0.8 }}
                      animate={{ opacity: 1, scale: 1 }}
                      transition={{ duration: 0.3, delay: 0.8 + index * 0.1 }}
                    >
                      <Badge variant="solid" size="2" className="text-sm">
                        {indicator}
                      </Badge>
                    </motion.div>
                  ))}
                </Flex>
              </Flex>
            )}
            <Separator size="4" />
            <div className="mt-3 w-full">
              <BookDescriptionAccordion
                description={book.description || 'No description available'}
                isOpen={isAccordionOpen}
                onToggle={() => setIsAccordionOpen(!isAccordionOpen)}
              />
            </div>
          </Flex>
        </Flex>
      </motion.div>
    </Card>
  );
};

export default RecommendationCard;
