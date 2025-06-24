import { motion } from "framer-motion";
import type { FC } from "react";
import { Badge, Flex } from "@radix-ui/themes";

interface AuthorBadgesProps {
  authors: string[];
}

const AuthorBadges: FC<AuthorBadgesProps> = ({ authors }) => {
  return (
    <Flex gap="1" asChild direction="column" align="end" justify="between">
      <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 0.3 }}>
        {authors.map((author, index) => (
          <Badge key={index} size="1" variant="soft" className="max-w-max">{author}</Badge>
        ))}
      </motion.div>
    </Flex>
  )
}

export default AuthorBadges;
