import axios, { AxiosError, type AxiosInstance, type AxiosResponse } from 'axios';
import type {
  ApiErrorResponse,
  Book,
  HealthCheckResponse,
  RecommendationRequest,
  RecommendationResponse,
  SearchHistoryResponse,
  PaginationParams,
  SearchHistoryEntry,
} from './types';
import apiConfig from './config';

// Generic success response type
interface ApiResponse<T> {
  data: T;
  error: null;
}

// Type for all possible API responses
export type ApiResult<T> = ApiResponse<T> | ApiErrorResponse;

class ApiClient {
  private client: AxiosInstance;

  constructor() {
    this.client = axios.create({
      baseURL: apiConfig.baseURL,
      headers: {
        'Content-Type': 'application/json',
        Accept: 'application/json',
      },
      // Timeout after 30 seconds
      timeout: 30000,
    });

    // Response interceptor for standardized error handling
    this.client.interceptors.response.use(
      (response: AxiosResponse) => {
        // Return the original response to maintain AxiosResponse type compatibility
        return response;
      },
      (error: AxiosError<ApiErrorResponse>) => {
        const defaultError = {
          code: 'UNKNOWN_ERROR',
          message: 'An unexpected error occurred',
        };

        // Handle network errors
        if (!error.response) {
          throw {
            error: {
              code: 'NETWORK_ERROR',
              message: 'Network error occurred. Please check your connection.',
            },
          };
        }

        // Handle timeout errors
        if (error.code === 'ECONNABORTED') {
          throw {
            error: {
              code: 'TIMEOUT_ERROR',
              message: 'Request timed out. Please try again.',
            },
          };
        }

        // Use error from API if available, otherwise use default
        const apiError = error.response.data?.error || defaultError;

        throw {
          error: apiError,
        };
      }
    );
  }

  // Health check endpoint
  async checkHealth(): Promise<ApiResult<HealthCheckResponse>> {
    return this.client.get(apiConfig.endpoints.health);
  }

  // Get book recommendations
  async getRecommendations(
    request: RecommendationRequest
  ): Promise<ApiResult<RecommendationResponse>> {
    const payload = {
      query: request.query,
      top_k: request.top_k || 10,
      user_id: request.user_id,
    };
    return this.client.post(apiConfig.endpoints.recommendations, payload);
  }

  // Get search history with pagination
  async getSearchHistory(params?: PaginationParams): Promise<ApiResult<SearchHistoryResponse>> {
    return this.client.get(apiConfig.endpoints.searchHistory, { params });
  }

  // Add search to history
  async addToSearchHistory(
    entry: Omit<SearchHistoryEntry, 'id' | 'created_at'>
  ): Promise<ApiResult<SearchHistoryEntry>> {
    return this.client.post(apiConfig.endpoints.searchHistory, entry);
  }

  // Delete search history entry
  async deleteSearchHistoryEntry(id: string): Promise<ApiResult<void>> {
    return this.client.delete(`${apiConfig.endpoints.searchHistory}/${id}`);
  }

  // Get book details
  async getBookDetails(id: string): Promise<ApiResult<Book>> {
    const response = await this.client.get(`${apiConfig.endpoints.books}/${id}`);
    if (response.data) {
      // Map the response to match our Book interface
      const book = response.data;
      return {
        data: {
          ...book,
          categories: book.categories || [],
          rating: book.rating || 0,
        },
        error: null,
      };
    }
    throw {
      error: {
        code: 'NOT_FOUND',
        message: 'Book not found',
      },
    };
  }

  // Clear all search history
  async clearSearchHistory(): Promise<ApiResult<void>> {
    return this.client.delete(apiConfig.endpoints.searchHistory);
  }

  // Get similar books
  async getSimilarBooks(bookId: string, limit: number = 5): Promise<ApiResult<Book[]>> {
    return this.client.get(`${apiConfig.endpoints.books}/${bookId}/similar`, {
      params: { limit },
    });
  }
}

// Export a singleton instance
export const apiClient = new ApiClient();
