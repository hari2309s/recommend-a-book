import { useEffect, useState } from 'react';
import { motion } from 'framer-motion';
import { fetchReommendations } from '@/api';
import type { Book } from '@/api/types';
import { Box, Button, Flex, Heading, TextField } from '@radix-ui/themes';
import "./App.css";
import { Search } from 'lucide-react';
import RecommendationList from '@/components/RecommendationList';
import FingerprintJS from '@fingerprintjs/fingerprintjs';

const App = () => {
  const [input, setInput] = useState('');
  const [recommendations, setRecommendations] = useState<Book[]>([]);
  const [loading, setLoading] = useState(false);
  const [deviceId, setDeviceId] = useState<string | null>(null);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setLoading(true);
    try {
      const data = await fetchReommendations(input, deviceId!);
      setRecommendations(data.recommendations);
    } catch (error) {
      console.error('Error fetching recommendations:', error);
    }
    setLoading(false);
  };

  useEffect(() => {
    const initializeFingerprint = async () => {
      const fp = await FingerprintJS.load();
      const result = await fp.get();
      setDeviceId(result.visitorId);
    };
    initializeFingerprint();
  }, []);

  return (
    <Box minHeight="100vh" minWidth="100vw" p="8" style={{ backgroundColor: 'var(--accent-1)' }} className='max-w-screen'>
      <Flex direction="column" gap="4" align="center">
        <Box minWidth="70%" p="4" style={{
          border: '1px dashed var(--accent-8)',
          textAlign: 'center', borderRadius: '6px', boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
          backgroundColor: 'var(--accent-2)', color: 'var(--accent-11)'
        }}>
          <motion.img src="/book-store.png" width={35} height={35} alt="Book Recommendation System"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ type: 'spring', duration: 1 }} />
          <Heading size="8" asChild>
            <motion.h1
              className="text-3xl font-bold mb-6"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ type: 'spring', duration: 1 }}
            >
              Book Recommendation System
            </motion.h1>
          </Heading>
        </Box>
        <Flex width="100%" p="4" gap={{ initial: '2', sm: '4' }} align="center" justify="between" direction={{ initial: 'column', sm: 'row' }} style={{
          border: '1px dashed var(--accent-8)',
          textAlign: 'center', borderRadius: '6px', boxShadow: '0 2px 4px rgba(0, 0, 0, 0.1)',
          backgroundColor: 'var(--accent-2)', color: 'var(--accent-11)',
        }} asChild>
          <motion.form onSubmit={handleSubmit} className="w-full max-w-md"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            transition={{ duration: 1 }}>
            <TextField.Root size="3" placeholder="Enter your book preferences" value={input}
              onChange={(e) => setInput(e.target.value)} className='w-full'>
              <TextField.Slot>
                <Search height="16" width="16" />
              </TextField.Slot>
            </TextField.Root>
            <Button variant="soft" type="submit" loading={loading} disabled={loading || input.trim() === ''} size="3" m="4">Get Recommendations</Button>
          </motion.form>
        </Flex>
        {recommendations.length > 0 && (
          <RecommendationList recommendations={recommendations} />
        )}
      </Flex>
    </Box>
  );
};

export default App;
