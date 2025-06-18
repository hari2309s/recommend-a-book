import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { fetchReommendations } from '@/api';
import type { Book } from '@/api/types';
import { Box, Button, Flex, Heading, ScrollArea, TextField } from '@radix-ui/themes';
import "./App.css";
import RecommendationCard from './components/RecommendationCard';
import { data } from './api/mockData';
import { Search } from 'lucide-react';

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
    <Box minHeight="100vh" minWidth="100vw" p="8" style={{ backgroundColor: 'var(--accent-3)' }} className='max-w-screen'>
      <Flex direction="column" gap="4" align="center">
        <Box minWidth="70%" p="4" style={{
          border: '1px solid var(--accent-8)',
          textAlign: 'center', borderRadius: '6px', boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
          backgroundColor: 'var(--accent-1)', color: 'var(--accent-11)'
        }}>
          <Heading size="8" asChild>
            <motion.h1
              className="text-3xl font-bold mb-6"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 1 }}
            >
              Book Recommendation System
            </motion.h1>
          </Heading>
        </Box>
        <Flex width="70%" p="4" gap="4" align="center" justify="between" style={{
          border: '1px solid var(--accent-8)',
          textAlign: 'center', borderRadius: '6px', boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
          backgroundColor: 'var(--accent-1)', color: 'var(--accent-11)',
        }} asChild>
          <motion.form onSubmit={handleSubmit} className="w-full max-w-md">
            <TextField.Root size="3" placeholder="Enter your book preferences" value={input}
              onChange={(e) => setInput(e.target.value)} className='w-full'>
              <TextField.Slot>
                <Search height="16" width="16" />
              </TextField.Slot>
            </TextField.Root>
            <Button variant="soft" type="submit" loading={loading} disabled={loading || input.trim() === ''} size="3" m="4">Get Recommendations</Button>
          </motion.form>
        </Flex>
        {recommendations.length > 0 ? (
          <Box asChild className='text-center'>
            <motion.div className="mt-6 w-full max-w-md" initial={{ opacity: 0 }} animate={{ opacity: 1 }} transition={{ duration: 1 }}>
              <Heading size="6">Recommended Books</Heading>
              <ScrollArea>
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
              </ScrollArea>
            </motion.div>
          </Box>) : (<RecommendationCard book={data.recommendations[6]} />)}
      </Flex>
    </Box>
  );
};

export default App;
