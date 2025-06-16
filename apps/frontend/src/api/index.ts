import type { Book } from '@/api/types';

export const fetchReommendations = async (
  searchText: string
): Promise<{ recommendations: Book[] }> => {
  const response = await fetch('http://localhost:3000/recommend', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ query: searchText }),
  });
  const data = await response.json();
  return data;
};
