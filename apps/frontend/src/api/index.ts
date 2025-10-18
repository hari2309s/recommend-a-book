import apiConfig from '@api/config';
import type {
  Book,
  RecommendationRequest,
  RecommendationResponse,
  ApiErrorResponse,
  ColdStartInfo,
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

// Cold start detection thresholds
const COLD_START_THRESHOLD_MS = 25000; // 25 seconds indicates potential cold start
const COLD_START_RETRY_DELAYS = [5000, 10000, 15000]; // Progressive delays for retries
const MAX_COLD_START_RETRIES = 3;

// Track if this is the first request
let isFirstRequest = true;

/**
 * Custom API error class for better error handling
 */
export class ApiError extends Error {
  status?: number;
  code?: string;
  retryable: boolean;
  coldStartInfo?: ColdStartInfo;

  constructor(
    message: string,
    options?: {
      status?: number;
      code?: string;
      retryable?: boolean;
      coldStartInfo?: ColdStartInfo;
    }
  ) {
    super(message);
    this.name = 'ApiError';
    this.status = options?.status;
    this.code = options?.code;
    this.retryable = options?.retryable ?? false;
    this.coldStartInfo = options?.coldStartInfo;
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

  static isColdStart(error: unknown): boolean {
    if (error instanceof ApiError) {
      return error.coldStartInfo?.isColdStart ?? false;
    }
    return false;
  }
}

/**
 * Detects if the request was affected by a cold start
 */
function detectColdStart(duration: number, error?: Error, isFirst: boolean = false): ColdStartInfo {
  // First request after page load is likely to hit a cold API
  if (isFirst && duration > 10000) {
    return {
      isColdStart: true,
      reason: 'first_request',
      duration,
    };
  }

  // Timeout errors are strong indicators of cold starts
  if (error?.name === 'AbortError') {
    return {
      isColdStart: true,
      reason: 'timeout',
      duration,
    };
  }

  // Network errors during initial connection
  if (error?.name === 'TypeError' && error.message.includes('Failed to fetch')) {
    return {
      isColdStart: true,
      reason: 'network_error',
      duration,
    };
  }

  // Slow response time indicates cold start
  if (duration > COLD_START_THRESHOLD_MS) {
    return {
      isColdStart: true,
      reason: 'slow_response',
      duration,
    };
  }

  return {
    isColdStart: false,
    duration,
  };
}

/**
 * Fetches book recommendations with cold start detection and retry logic
 */
export async function fetchRecommendations(
  searchText: string,
  topK: number = 100,
  options: {
    useCache?: boolean;
    signal?: AbortSignal;
    onColdStart?: (info: ColdStartInfo) => void;
    onRetry?: (attempt: number, maxRetries: number) => void;
  } = {}
): Promise<RecommendationResponse> {
  const { useCache = true, signal, onColdStart, onRetry } = options;
  const trimmedQuery = searchText.trim();
  const cacheKey = `${trimmedQuery}:${topK}`;
  const wasFirstRequest = isFirstRequest;
  isFirstRequest = false;

  // Check cache if enabled
  if (useCache) {
    const cachedResponse = responseCache.get(cacheKey);
    const now = Date.now();

    if (cachedResponse && now < cachedResponse.expiry) {
      return cachedResponse.data;
    }
  }

  let lastError: Error | null = null;
  let attempt = 0;

  while (attempt <= MAX_COLD_START_RETRIES) {
    const startTime = Date.now();

    // Create abort controller with extended timeout for cold starts
    const controller = new AbortController();
    const timeoutMs = attempt === 0 ? apiConfig.requestTimeout : apiConfig.requestTimeout * 2;
    const timeoutId = setTimeout(() => controller.abort(), timeoutMs);

    // Use provided signal or our controller's signal
    const requestSignal = signal || controller.signal;

    try {
      if (attempt > 0) {
        // Use progressive backoff for cold start retries
        const backoffTime = COLD_START_RETRY_DELAYS[attempt - 1] || 15000;

        // Notify about retry
        if (onRetry) {
          onRetry(attempt, MAX_COLD_START_RETRIES);
        }

        await new Promise((resolve) => setTimeout(resolve, backoffTime));
      }

      // Construct URL
      const url = `${apiConfig.baseURL}${apiConfig.endpoints.recommendations}`;

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

      const duration = Date.now() - startTime;

      // Detect potential cold start based on response time
      const coldStartInfo = detectColdStart(duration, undefined, wasFirstRequest && attempt === 0);

      if (coldStartInfo.isColdStart && onColdStart && attempt === 0) {
        onColdStart(coldStartInfo);
      }

      if (!response.ok) {
        const status = response.status;
        let errorMessage: string;
        let isRetryable = false;

        try {
          const errorData = (await response.json()) as ApiErrorResponse;
          errorMessage = errorData.error?.message || `API request failed with status ${status}`;
        } catch {
          errorMessage = `API request failed with status ${status}`;
        }

        // Server errors, rate limiting, and gateway timeouts are retryable
        isRetryable = status >= 500 || status === 429 || status === 504;

        throw new ApiError(errorMessage, {
          status,
          retryable: isRetryable,
          coldStartInfo: isRetryable ? coldStartInfo : undefined,
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
        relevance_indicators: book.relevance_indicators || [],
        confidence_score: book.confidence_score || 0,
      }));

      const result = {
        recommendations,
        semantic_tags: data.semantic_tags || [],
      };

      // Store in cache if caching is enabled
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

      clearTimeout(timeoutId);
      return result;
    } catch (error) {
      clearTimeout(timeoutId);
      const duration = Date.now() - startTime;
      lastError = error instanceof Error ? error : new Error('Unknown error occurred');

      // Detect cold start from error
      const coldStartInfo = detectColdStart(duration, lastError, wasFirstRequest && attempt === 0);

      // Check if we should retry for cold start
      if (attempt < MAX_COLD_START_RETRIES && coldStartInfo.isColdStart) {
        if (onColdStart && attempt === 0) {
          onColdStart(coldStartInfo);
        }
        attempt++;
        continue;
      }

      // No more retries or non-cold-start error
      if (error instanceof ApiError) {
        throw error;
      } else if (error instanceof Error) {
        if (error.name === 'AbortError') {
          throw new ApiError(`Request timed out after ${timeoutMs}ms`, {
            code: 'TIMEOUT',
            retryable: true,
            coldStartInfo,
          });
        } else if (error.name === 'TypeError' && error.message.includes('Failed to fetch')) {
          throw new ApiError(
            'Network error: API server may be unavailable or experiencing cold start',
            {
              code: 'NETWORK_ERROR',
              retryable: true,
              coldStartInfo,
            }
          );
        }
        throw new ApiError(error.message, { coldStartInfo });
      }
      throw new ApiError('An unexpected error occurred', { coldStartInfo });
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
 */
export function invalidateCache(query?: string): void {
  if (query) {
    for (const key of responseCache.keys()) {
      if (key.startsWith(query.trim())) {
        responseCache.delete(key);
      }
    }
  } else {
    responseCache.clear();
  }
}
