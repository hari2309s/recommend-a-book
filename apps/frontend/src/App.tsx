import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import "@radix-ui/themes/styles.css";
import { fetchReommendations } from '@/api';
import type { Book } from '@/api/types';
import { Box, Flex } from '@radix-ui/themes';

const App = () => {
  const [input, setInput] = useState('');
  const [recommendations, setRecommendations] = useState<Book[]>([]);
  const [loading, setLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      const data = await fetchReommendations(input);
      setRecommendations(data.recommendations);
    } catch (error) {
      console.error('Error fetching recommendations:', error);
    }
    setLoading(false);
  };

  useEffect(() => {
    console.log('recommendations', recommendations);
  }, [recommendations]);

  return (
    <Box minHeight="100vh" minWidth="100vw" p="4">
      <Flex direction="column" gap="4" align="center">
        <motion.h1
          className="text-3xl font-bold mb-6"
          initial={{ opacity: 0 }}
          animate={{ opacity: 1 }}
          transition={{ duration: 1 }}
        >
          Book Recommendation System
        </motion.h1>
        <form onSubmit={handleSubmit} className="w-full max-w-md">
          <textarea
            className="w-full p-3 border rounded-lg mb-4"
            rows={4}
            placeholder="Enter your book preferences (e.g., 'I like sci-fi novels with strong female leads')"
            value={input}
            onChange={(e) => setInput(e.target.value)}
          ></textarea>
          <button
            type="submit"
            className="w-full bg-blue-500 text-white p-3 rounded-lg hover:bg-blue-600 disabled:bg-gray-400"
            disabled={loading}
          >
            {loading ? 'Loading...' : 'Get Recommendations'}
          </button>
        </form>
        {recommendations.length > 0 && (
          <div className="mt-6 w-full max-w-md">
            <h2 className="text-2xl font-semibold mb-4">Recommended Books</h2>
            <ul className="space-y-4">
              {recommendations.map((book, index) => (
                <li key={index} className="flex items-start">
                  {book.thumbnail && (
                    <img src={book.thumbnail} alt={book.title} className="w-16 h-24 mr-4 object-cover rounded" />
                  )}
                  <div>
                    <strong className="text-lg">{book.title}</strong> by {book.author}
                    <p className="text-sm text-gray-600">{book.description}</p>
                    {book.rating && <p className="text-sm text-yellow-500">Rating: {book.rating}</p>}
                  </div>
                </li>
              ))}
            </ul>
          </div>
        )}
      </Flex>
    </Box>
  );
};

export default App;
