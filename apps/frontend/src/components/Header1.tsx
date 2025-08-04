import React, { useState } from 'react';
import { motion, useScroll } from 'framer-motion';
import { Heading } from '@radix-ui/themes';

const Header: React.FC = () => {
  const [isSticky, setIsSticky] = useState<boolean>(false);
  const { scrollY } = useScroll();

  scrollY.onChange((latest) => {
    if (latest > 50) {
      setIsSticky(true);
    } else {
      setIsSticky(false);
    }
  });

  return (
    <motion.header
      className={`fixed top-4 left-4 right-4 bg-green-500/40 backdrop-blur-lg transition-all duration-300
        ease-in-out z-50 border border-dashed border-green-300/20 ${
          isSticky ? 'shadow-2xl shadow-green-500/20' : 'shadow-lg shadow-green-500/10'
        } rounded-md min-h-[80px] w-[70%] flex`}
      style={{
        backdropFilter: 'blur(20px)',
        WebkitBackdropFilter: 'blur(20px)',
        background: 'rgba(34, 197, 94, 0.4)',
        boxShadow: isSticky
          ? '0 25px 50px -12px rgba(34, 197, 94, 0.25), 0 8px 16px -8px rgba(34, 197, 94, 0.1)'
          : '0 10px 25px -5px rgba(34, 197, 94, 0.1), 0 4px 6px -2px rgba(34, 197, 94, 0.05)',
        borderRadius: '5px',
        borderColor: 'var(--accent-8)',
      }}
      initial={{ opacity: 0, y: -20 }}
      animate={{ opacity: 1, y: 0 }}
      transition={{ duration: 0.6, ease: 'easeOut' }}
    >
      <div className="container px-2 py-4 flex items-center justify-around">
        <motion.div className="flex items-center justify-around space-x-4 w-full">
          <motion.img
            src="/book-store.png"
            width={35}
            height={35}
            alt="Book Recommendation System"
            initial={{ opacity: 0, rotate: -180 }}
            animate={{ opacity: 1, rotate: 0 }}
            transition={{
              type: 'spring',
              duration: 1,
              stiffness: 100,
              damping: 10,
            }}
            whileHover={{
              rotate: 360,
              scale: 1.1,
              transition: { duration: 0.6 },
            }}
          />

          <Heading size="8" asChild>
            <motion.h1
              className="text-3xl font-bold mb-6 text-[var(--accent-11)]"
              initial={{ opacity: 0, y: 20 }}
              animate={{ opacity: 1, y: 0 }}
              transition={{
                type: 'spring',
                duration: 1,
                delay: 0.3,
              }}
            >
              Book Recommendation System
            </motion.h1>
          </Heading>
        </motion.div>
      </div>
    </motion.header>
  );
};

export default Header;
