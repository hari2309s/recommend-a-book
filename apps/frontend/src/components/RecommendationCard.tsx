import type { Book } from "@/api/types";
import { Card, Heading, Badge, Flex, Text, Separator } from "@radix-ui/themes"
import { motion } from "framer-motion";
import { Star } from "lucide-react";
import BookThumbnail from "@/components/BookThumbnail";
import type { FC } from "react";
import AuthorBadges from "@/components/AuthorBadges";

interface RecommendationCardProps {
  book: Book;
}

const RecommendationCard: FC<RecommendationCardProps> = ({ book }) => {
  return (
    <Card size={{ initial: '1', sm: '2', md: '3' }}
      style={{
        textAlign: 'right', backgroundColor: 'var(--accent-6)',
        color: 'var(--accent-11)', height: '290px',
      }}>
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 1 }}>
        <Flex gap="2" direction='column' align='stretch'>
          <Flex gap='2' direction="row" justify='between'>
            <BookThumbnail src={book.thumbnail} alt={book.title} className="mr-4" />
            <Flex gap='2' direction='column' align='end' className="max-h-48! text-ellipsis overflow-hidden">
              <Heading size='6' asChild className="text-ellipsis overflow-hidden">
                <motion.h3>{book.title}</motion.h3>
              </Heading>
              <AuthorBadges authors={book.author.split(';')} />
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
