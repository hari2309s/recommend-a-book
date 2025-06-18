import { Box } from "@radix-ui/themes";
import { motion } from "framer-motion";
import type { Book } from "@/api/types";
import { Heading, ScrollArea } from "@radix-ui/themes";
import RecommendationCard from "./RecommendationCard";

interface RecommendationListProps {
  recommendations: Book[];
}

const RecommendationList = ({ recommendations }: RecommendationListProps) => {
  return (
    <Box asChild width='540px' p='4'>
      <motion.div>
        <Heading size="6">Recommended Books</Heading>
        <ScrollArea>
          <motion.ul className="space-y-4">
            {recommendations.map((book, index) => (
              <motion.li key={index} className="flex items-start">
                <RecommendationCard book={book} />
              </motion.li>
            ))}
          </motion.ul>
        </ScrollArea>
      </motion.div>
    </Box>
  )
}

export default RecommendationList;
