import type { Book } from "@/api/types";
import { Card, Heading, Badge, Flex, Text, Separator } from "@radix-ui/themes"
import { motion } from "framer-motion";
import { Star } from "lucide-react";
import BookThumbnail from "@/components/BookThumbnail";
import type { FC } from "react";

interface RecommendationCardProps {
  book: Book;
}

const RecommendationCard: FC<RecommendationCardProps> = ({ book }) => {
  return (
    <Card size={{ initial: '1', sm: '2', md: '3' }} style={{
      textAlign: 'right',
      backgroundColor: 'var(--accent-6)', color: 'var(--accent-11)',
      height: '290px',
    }}>
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 1 }}>
        <Flex gap="2" direction='column' align='stretch'>
          <Flex gap='2' direction={{ initial: 'column', sm: 'row' }} justify='between'>
            <BookThumbnail src={book.thumbnail} alt={book.title} className="mr-4" />
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
          <Flex direction='column' justify='center' align='end' gap='2' className="align-normal">
            {book.categories && <Badge size='3' variant='surface' className="max-w-max">
              {book.categories}</Badge>}
            <Flex gap='3' justify='between' className="w-full">
              <Flex gap='3' justify='center'>
                <Star fill="green" />
                <Text>{book.rating} / {book.ratingsCount}</Text>
              </Flex>
              {book.publishedYear && <Text className="italic">{book.publishedYear}</Text>}
            </Flex>
            <Separator size='4' />
          </Flex>
        </Flex>
      </motion.div>
    </Card >
  )
}

export default RecommendationCard;
