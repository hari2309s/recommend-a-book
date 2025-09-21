import apiConfig from '@api/config';
import type {
  Book,
  RecommendationRequest,
  RecommendationResponse,
  ApiErrorResponse,
} from '@api/types';

// Cache for API responses
interface CacheEntry {
  data: RecommendationResponse;
  timestamp: number;
  expiry: number;
}

// Cache storage with TTL (5 minutes default)
const responseCache = new Map<string, CacheEntry>();
const CACHE_TTL_MS = 5 * 60 * 1000; // 5 minutes cache duration
const MAX_RETRIES = 2;

/**
 * Custom API error class for better error handling
 */
export class ApiError extends Error {
  status?: number;
  code?: string;
  retryable: boolean;

  constructor(message: string, options?: { status?: number; code?: string; retryable?: boolean }) {
    super(message);
    this.name = 'ApiError';
    this.status = options?.status;
    this.code = options?.code;
    this.retryable = options?.retryable ?? false;
  }

  static isRetryable(error: unknown): boolean {
    if (error instanceof ApiError) {
      return error.retryable;
    }
    // Network errors and timeouts are generally retryable
    if (error instanceof Error) {
      return (
        error.name === 'AbortError' ||
        (error.name === 'TypeError' && error.message.includes('Failed to fetch'))
      );
    }
    return false;
  }
}

/**
 * Fetches book recommendations based on the provided search criteria
 * @param searchText - The search query text
 * @param topK - Number of recommendations to return (default: 55)
 * @param options - Additional options like cache control and retry settings
 * @returns Promise with recommendations response
 * @throws ApiError if the API request fails
 */
export async function fetchRecommendations(
  searchText: string,
  topK: number = 100, // Default to 100 results for better coverage
  options: {
    useCache?: boolean;
    retries?: number;
    signal?: AbortSignal;
    limitResults?: boolean; // Option to limit results in the frontend
  } = {}
): Promise<RecommendationResponse> {
  const { useCache = true, retries = MAX_RETRIES, signal, limitResults = false } = options;
  const trimmedQuery = searchText.trim();
  const cacheKey = `${trimmedQuery}:${topK}`;

  // Check cache if enabled
  if (useCache) {
    const cachedResponse = responseCache.get(cacheKey);
    const now = Date.now();

    if (cachedResponse && now < cachedResponse.expiry) {
      return cachedResponse.data;
    }
  }

  // Create abort controller for timeout if signal not provided
  let timeoutId: ReturnType<typeof setTimeout> | undefined;
  const controller = new AbortController();

  if (!signal) {
    timeoutId = setTimeout(() => controller.abort(), apiConfig.requestTimeout);
  }

  // Use provided signal or our controller's signal
  const requestSignal = signal || controller.signal;

  let lastError: Error | null = null;
  let attempt = 0;

  while (attempt <= retries) {
    try {
      if (attempt > 0) {
        // Add exponential backoff for retries
        const backoffTime = Math.min(100 * Math.pow(2, attempt), 2000);
        await new Promise((resolve) => setTimeout(resolve, backoffTime));
      }

      // Construct URL correctly by combining baseURL and endpoint
      const baseUrl = apiConfig.baseURL || '';
      const endpoint = apiConfig.endpoints.recommendations;
      const url = baseUrl + endpoint;

      const response = await fetch(url, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          Accept: 'application/json',
        },
        body: JSON.stringify({
          query: trimmedQuery,
          topK,
        } satisfies RecommendationRequest),
        signal: requestSignal,
      });

      if (!response.ok) {
        // Handle different HTTP error status codes
        const status = response.status;
        let errorMessage: string;
        let isRetryable = false;

        try {
          const errorData = (await response.json()) as ApiErrorResponse;
          errorMessage = errorData.error?.message || `API request failed with status ${status}`;
        } catch {
          errorMessage = `API request failed with status ${status}`;
        }

        // Determine if error is retryable based on status code
        isRetryable = status >= 500 || status === 429; // Server errors and rate limiting

        throw new ApiError(errorMessage, {
          status,
          retryable: isRetryable,
        });
      }

      const data = await response.json();

      // Validate response structure
      if (!data || !Array.isArray(data.recommendations)) {
        throw new ApiError('Invalid API response format: missing recommendations array');
      }

      // Process and normalize book data
      const recommendations = data.recommendations.map((book: Partial<Book>) => ({
        id: book.id || '',
        title: book.title,
        author: book.author,
        description: book.description,
        categories: book.categories || [],
        thumbnail: book.thumbnail,
        rating: book.rating || 0,
        ratings_count: book.ratings_count || 0,
        year: book.year || '',
        published_year: book.published_year || '',
        isbn: book.isbn,
        page_count: book.page_count,
        language: book.language,
        publisher: book.publisher,
      }));

      // Create result object - don't limit results unless specifically requested
      const result = {
        recommendations: limitResults ? recommendations.slice(0, topK) : recommendations,
      };

      // Store in cache if caching is enabled - always store the full result set
      if (useCache) {
        responseCache.set(cacheKey, {
          data: result,
          timestamp: Date.now(),
          expiry: Date.now() + CACHE_TTL_MS,
        });

        // Clean up old cache entries if cache is getting large
        if (responseCache.size > 50) {
          cleanupCache();
        }
      }

      return result;
    } catch (error) {
      lastError = error instanceof Error ? error : new Error('Unknown error occurred');

      // Check if we should retry
      if (attempt < retries && ApiError.isRetryable(error)) {
        attempt++;
        continue;
      }

      // No more retries or non-retryable error
      if (error instanceof ApiError) {
        throw error;
      } else if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new ApiError(`Request timed out after ${apiConfig.requestTimeout}ms`, {
            code: 'TIMEOUT',
            retryable: true,
          });
        } else if (error.name === 'TypeError' && error.message.includes('Failed to fetch')) {
          throw new ApiError('Network error: API server may be unavailable or CORS issue', {
            code: 'NETWORK_ERROR',
            retryable: true,
          });
        }
        throw new ApiError(error.message);
      }
      throw new ApiError('An unexpected error occurred');
    } finally {
      if (timeoutId) clearTimeout(timeoutId);
    }
  }

  // This should never be reached due to the throw in the catch block
  throw lastError || new ApiError('Unknown error occurred during API request');
}

/**
 * Cleans up expired cache entries
 */
function cleanupCache(): void {
  const now = Date.now();
  for (const [key, entry] of responseCache.entries()) {
    if (now > entry.expiry) {
      responseCache.delete(key);
    }
  }
}

/**
 * Invalidates the cache for a specific query or all queries
 * @param query - Optional query to invalidate, if not provided all cache is cleared
 * @param preserveResults - Whether to preserve result data but mark as expired
 */
export function invalidateCache(query?: string, preserveResults: boolean = false): void {
  if (query) {
    // Remove all cache entries that start with this query
    for (const key of responseCache.keys()) {
      if (key.startsWith(query.trim())) {
        if (preserveResults) {
          // Mark as expired but keep data (useful for transitions)
          const entry = responseCache.get(key);
          if (entry) {
            responseCache.set(key, {
              ...entry,
              expiry: Date.now() - 1, // Set as expired
            });
          }
        } else {
          // Completely remove cache entry
          responseCache.delete(key);
        }
      }
    }
  } else {
    // Clear all cache
    responseCache.clear();
  }
}
