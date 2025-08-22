import apiConfig from '@api/config';
import type {
  Book,
  RecommendationRequest,
  RecommendationResponse,
  ApiErrorResponse,
} from '@api/types';

/**
 * Fetches book recommendations based on the provided search criteria
 * @param searchText - The search query text
 * @param deviceId - Unique identifier for the user/device
 * @param topK - Number of recommendations to return (default: 10)
 * @returns Promise with recommendations response
 * @throws Error if the API request fails
 */
export async function fetchRecommendations(
  searchText: string,
  deviceId: string,
  topK: number = 10
): Promise<RecommendationResponse> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), apiConfig.requestTimeout);

  try {
    const response = await fetch(`${apiConfig.baseURL}${apiConfig.endpoints.recommendations}`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      body: JSON.stringify({
        query: searchText.trim(),
        user_id: deviceId,
        topK,
      } satisfies RecommendationRequest),
      signal: controller.signal,
    });

    if (!response.ok) {
      const errorData = (await response.json()) as ApiErrorResponse;
      throw new Error(
        errorData.error?.message || `API request failed with status ${response.status}`
      );
    }

    const data = await response.json();

    // Validate response structure
    if (!data || !Array.isArray(data.recommendations)) {
      throw new Error('Invalid API response format');
    }

    // Ensure all books have required fields
    const recommendations = data.recommendations.map((book: Partial<Book>) => ({
      id: book.id || '',
      title: book.title,
      author: book.author,
      description: book.description,
      categories: book.categories || [],
      thumbnail: book.thumbnail,
      rating: book.rating || 0,
      year: book.year,
      isbn: book.isbn,
      page_count: book.page_count,
      language: book.language,
      publisher: book.publisher,
    }));

    return {
      recommendations,
      user_id: data.user_id || deviceId,
    };
  } catch (error) {
    if (error instanceof Error) {
      if (error.name === 'AbortError') {
        throw new Error('Request timed out');
      }
      throw error;
    }
    throw new Error('An unexpected error occurred');
  } finally {
    clearTimeout(timeoutId);
  }
}
