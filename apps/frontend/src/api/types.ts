// Types for API responses and requests

// Book type returned by the backend
export interface Book {
  id: string;
  title?: string;
  author?: string;
  description?: string;
  thumbnail?: string;
  categories: string[];
  rating: number;
  published_year?: number;
  ratings_count?: number;
  similarity_score?: number;
}

// Search/Recommendation request
export interface RecommendationRequest {
  query: string;
  top_k?: number;
  user_id?: string;
}

// Recommendation response
export interface RecommendationResponse {
  recommendations: Array<Book>;
  query: string;
  created_at: string;
}

// Search history entry
export interface SearchHistoryEntry {
  id: string;
  query: string;
  created_at: string;
  recommendations: Book[];
}

// Search history response
export interface SearchHistoryResponse {
  history: SearchHistoryEntry[];
  total: number;
}

// Error response from the backend
export interface ApiErrorResponse {
  error: {
    code: string;
    message: string;
    details?: Record<string, any>;
  };
}

// Health check response
export interface HealthCheckResponse {
  status: 'ok' | 'error';
  version: string;
  timestamp: string;
}

// Pagination parameters
export interface PaginationParams {
  page: number;
  per_page: number;
}

// Generic paginated response
export interface PaginatedResponse<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}
