import React, { useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { ChevronDown } from 'lucide-react';

type AccordionItem = {
  title: string;
  content: string;
};

type AccordionProps = {
  items: AccordionItem[];
};

const Accordion: React.FC<AccordionProps> = ({ items }) => {
  const [openIndex, setOpenIndex] = useState<number | null>(null);

  return (
    <div className="w-full">
      {items.map((item, index) => (
        <motion.div
          key={index}
          className="mb-2"
          initial={{ opacity: 0, y: 10 }}
          animate={{ opacity: 1, y: 0 }}
          transition={{
            duration: 0.3,
            delay: index * 0.1,
          }}
        >
          <motion.button
            className="w-full text-sm text-green-800 bg-green-200 px-3 py-2 rounded hover:bg-green-300 focus:outline-none focus:ring-2 focus:ring-green-500 transition-colors duration-200 flex items-center justify-between"
            whileHover={{
              scale: 1.02,
              backgroundColor: '#bbf7d0',
            }}
            whileTap={{ scale: 0.98 }}
            onClick={() => setOpenIndex(openIndex === index ? null : index)}
          >
            <span>{item.title}</span>
            <motion.div
              animate={{ rotate: openIndex === index ? 180 : 0 }}
              transition={{ duration: 0.3 }}
            >
              <ChevronDown size={16} />
            </motion.div>
          </motion.button>
          <AnimatePresence>
            {openIndex === index && (
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
                className="mt-2 text-sm text-green-900 overflow-hidden bg-green-50 rounded-md p-3"
              >
                <motion.div
                  initial={{ opacity: 0 }}
                  animate={{ opacity: 1 }}
                  transition={{ duration: 0.3, delay: 0.1 }}
                >
                  {item.content}
                </motion.div>
              </motion.div>
            )}
          </AnimatePresence>
        </motion.div>
      ))}
    </div>
  );
};

export default Accordion;
