import type { Book } from "@/api/types";
import { Box, Card, Heading, Badge, Flex, Text, Separator } from "@radix-ui/themes"
import { motion } from "framer-motion";
import { Star } from "lucide-react";

interface RecommendationCardProps {
  book: Book;
}

const RecommendationCard = ({ book }: RecommendationCardProps) => {
  return <Box>
    <Card size="3" asChild style={{
      border: '1px solid var(--accent-8)',
      textAlign: 'right', boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
      backgroundColor: 'var(--accent-3)', color: 'var(--accent-11)',
    }}>
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 1 }}>
        <Flex gap="2" direction='column' align='stretch'>
          <Flex gap='2' justify='between'>
            <motion.img src={book.thumbnail} alt={book.title} className="w-16 h-24 mr-4 object-cover rounded" />
            <Flex direction='column' align='end'>
              <Heading size='6' asChild className="text-center">
                <motion.h2>{book.title}</motion.h2>
              </Heading>
              <Text size='5' asChild className="text-center">
                <motion.span>{book.author && `by ${book.author.split(';').join(', ')}`}</motion.span>
              </Text>
            </Flex>
          </Flex>
          <Separator size='4' />
          <Flex direction='column' align='end' gap='2'>
            {book.categories && <Badge>{book.categories}</Badge>}
            <Flex gap='3' justify='end'>
              <Star fill="green" />
              <Text>{book.rating} / {book.ratingsCount}</Text>
            </Flex>
            {book.publishedYear && <Text>{book.publishedYear}</Text>}
          </Flex>
          <Text align='left'>{book.description}</Text>
        </Flex>
      </motion.div>
    </Card>
  </Box>
}

export default RecommendationCard;
