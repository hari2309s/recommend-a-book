import { motion } from 'framer-motion';
import React from 'react';
import { Badge, Flex } from '@radix-ui/themes';

type AuthorBadgesProps = {
  authors: string[];
};

const AuthorBadges: React.FC<AuthorBadgesProps> = ({ authors }) => {
  const containerVariants = {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        staggerChildren: 0.1,
        delayChildren: 0.2,
      },
    },
  };

  const badgeVariants = {
    hidden: {
      opacity: 0,
      y: 10,
      scale: 0.8,
    },
    visible: {
      opacity: 1,
      y: 0,
      scale: 1,
      transition: {
        type: 'spring' as const,
        stiffness: 200,
        damping: 15,
      },
    },
  };

  return (
    <Flex gap="1" asChild direction="column" align="end" justify="between">
      <motion.div variants={containerVariants} initial="hidden" animate="visible">
        {authors.map((author, index) => (
          <motion.div key={index} variants={badgeVariants}>
            <Badge
              size="1"
              variant="soft"
              className="max-w-max hover:bg-green-200 transition-colors duration-200"
            >
              {author}
            </Badge>
          </motion.div>
        ))}
      </motion.div>
    </Flex>
  );
};

export default AuthorBadges;
