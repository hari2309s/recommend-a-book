/**
 * Represents a book with its metadata
 */
export interface Book {
  id: string;
  title?: string;
  author?: string;
  description?: string;
  categories: string[];
  thumbnail?: string;
  rating: number;
  ratings_count?: number;
  published_year?: number;
  year?: number;
  isbn?: string;
  page_count?: number;
  language?: string;
  publisher?: string;
  explanation?: string;
}

/**
 * Request parameters for book recommendations
 */
export interface RecommendationRequest {
  query: string;
  topK?: number;
}

/**
 * Response from the recommendations API
 */
export interface RecommendationResponse {
  recommendations: Book[];
}

/**
 * API error response structure
 */
export interface ApiErrorResponse {
  error: {
    message: string;
    details?: Record<string, unknown>;
  };
}

/**
 * Cold start detection result
 */
export interface ColdStartInfo {
  isColdStart: boolean;
  reason?: 'timeout' | 'slow_response' | 'first_request' | 'network_error';
  duration?: number;
}
