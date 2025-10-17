import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown } from 'lucide-react';
import { Text } from '@radix-ui/themes';

type BookDescriptionAccordionProps = {
  description: string;
  isOpen: boolean;
  onToggle: () => void;
};

const BookDescriptionAccordion: React.FC<BookDescriptionAccordionProps> = ({
  description,
  isOpen,
  onToggle,
}) => {
  if (!description || description.trim() === '') {
    return null;
  }

  return (
    <div className="w-full">
      <motion.button
        className="w-full px-3 py-2.5 rounded-[5px] border border-[var(--espresso)]
        hover:bg-[var(--vivid-ember)] hover:cursor-pointer focus:outline-none focus:ring-2
        focus:ring-[var(--espresso)] focus:ring-offset-1 transition-all duration-200 flex
        items-center justify-between shadow-sm text-[var(--espresso)] text-lg"
        style={{ backgroundColor: 'var(--soft-sand)' }}
        whileHover={{
          scale: 1.01,
          backgroundColor: 'var(--espresso)',
          color: 'var(--soft-sand)',
        }}
        whileTap={{
          scale: 0.99,
          backgroundColor: 'var(--espresso)',
          color: 'var(--soft-sand)',
        }}
        onClick={onToggle}
      >
        <Text>Description</Text>
        <motion.div
          animate={{ rotate: isOpen ? 180 : 0 }}
          transition={{ duration: 0.3 }}
          className="text-[var(--espresso)] text-lg hover:text-[var(--soft-sand)]"
        >
          <ChevronDown size={20} className="hover:text-[var(--soft-sand)]!" />
        </motion.div>
      </motion.button>
      <AnimatePresence>
        {isOpen && (
          <motion.div
            initial={{ height: 0, opacity: 0, y: -10 }}
            animate={{ height: 'auto', opacity: 1, y: 0 }}
            exit={{ height: 0, opacity: 0, y: -10 }}
            transition={{
              duration: 0.4,
              ease: 'easeInOut',
              type: 'spring',
              stiffness: 100,
              damping: 15,
            }}
            className="overflow-hidden rounded-[5px] p-4 bg-[var(--soft-sand)]
            text-[var(--espresso)] mt-[10px]"
          >
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.3, delay: 0.1 }}
              className="text-sm leading-[1.6] p-[10px]"
            >
              {description}
            </motion.div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
};

export default BookDescriptionAccordion;
