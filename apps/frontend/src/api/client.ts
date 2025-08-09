import axios, { AxiosError, type AxiosInstance } from 'axios';
import {
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
      (response) => ({
        data: response.data,
        error: null,
      }),
      (error: AxiosError<ApiErrorResponse>) => {
        const defaultError = {
          code: 'UNKNOWN_ERROR',
          message: 'An unexpected error occurred',
        };

        // Handle network errors
        if (!error.response) {
          return {
            data: null,
            error: {
              code: 'NETWORK_ERROR',
              message: 'Network error occurred. Please check your connection.',
            },
          };
        }

        // Handle timeout errors
        if (error.code === 'ECONNABORTED') {
          return {
            data: null,
            error: {
              code: 'TIMEOUT_ERROR',
              message: 'Request timed out. Please try again.',
            },
          };
        }

        // Use error from API if available, otherwise use default
        const apiError = error.response.data?.error || defaultError;

        return {
          data: null,
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
    return this.client.post(apiConfig.endpoints.recommendations, request);
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
    return this.client.get(`${apiConfig.endpoints.books}/${id}`);
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
