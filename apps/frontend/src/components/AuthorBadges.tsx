import { motion } from 'framer-motion';
import React from 'react';
import { Badge, Flex } from '@radix-ui/themes';
import { badgeContainerVariants, badgeVariants } from '@/utils/animations';

type AuthorBadgesProps = {
  authors: string[];
};

const AuthorBadges: React.FC<AuthorBadgesProps> = ({ authors }) => {
  return (
    <Flex gap="1" asChild direction="column" align="end" justify="between">
      <motion.div variants={badgeContainerVariants} initial="hidden" animate="visible">
        {authors.map((author, index) => (
          <Badge
            asChild
            size="1"
            variant="soft"
            className="max-w-max hover:bg-green-200 transition-colors duration-200"
          >
            <motion.div key={index} variants={badgeVariants}>
              {author}
            </motion.div>
          </Badge>
        ))}
      </motion.div>
    </Flex>
  );
};

export default AuthorBadges;
