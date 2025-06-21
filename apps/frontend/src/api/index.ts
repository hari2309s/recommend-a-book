import type { Book } from '@/api/types';

export const fetchReommendations = async (
  searchText: string,
  deviceId: string
): Promise<{ recommendations: Book[] }> => {
  const response = await fetch(`${import.meta.env.VITE_RECOMMEND_A_BOOK_API_BASE_URL}/recommend`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ query: searchText, user_id: deviceId }),
  });
  const data = await response.json();
  return data;
};
