import type { Book } from "@/api/types";
import { Card, Heading, Badge, Flex, Text, Separator } from "@radix-ui/themes"
import { motion } from "framer-motion";
import { Star } from "lucide-react";
import BookDescriptionAccordion from "./BookDescriptionAccordion";

interface RecommendationCardProps {
  book: Book;
}

const RecommendationCard = ({ book }: RecommendationCardProps) => {
  return (
    <Card size={{ initial: '1', sm: '2', md: '3' }} style={{
      border: '1px solid var(--accent-8)',
      textAlign: 'right', boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
      backgroundColor: 'var(--accent-2)', color: 'var(--accent-11)',
    }}>
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 1 }}>
        <Flex gap="2" direction='column' align='stretch'>
          <Flex gap='2' direction={{ initial: 'column', sm: 'row' }} justify='between'>
            <motion.img src={book.thumbnail} alt={book.title} className="w-16 h-24 mr-4" />
            <Flex direction='column' align='end'>
              <Heading size='6' asChild>
                <motion.h2>{book.title}</motion.h2>
              </Heading>
              <Text size='4' asChild>
                <motion.span>{book.author && `by ${book.author.split(';').join(', ')}`}</motion.span>
              </Text>
            </Flex>
          </Flex>
          <Separator size='4' />
          <Flex justify='between' gap='2'>
            {book.categories && <Badge size='3' variant='surface'>{book.categories}</Badge>}
            <Flex gap='3' justify='end'>
              <Star fill="green" />
              <Text>{book.rating} / {book.ratingsCount}</Text>
            </Flex>
            {book.publishedYear && <Text className="italic">{book.publishedYear}</Text>}
          </Flex>
          <Separator size='4' />
          <BookDescriptionAccordion description={book.description} />
        </Flex>
      </motion.div>
    </Card>
  )
}

export default RecommendationCard;
