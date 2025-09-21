/**
 * API Configuration
 *
 * In development: Uses Vite's proxy configuration with relative paths
 * In production: Uses the absolute API URL from environment variables
 */

// API configuration types
type ApiEndpoints = {
  recommendations: string;
};

type ApiConfig = {
  baseURL: string;
  endpoints: ApiEndpoints;
  requestTimeout: number;
};

// Determine the base URL based on environment
const getBaseUrl = (): string => {
  // In production, use the environment variable or empty string
  if (import.meta.env.PROD) {
    return import.meta.env.VITE_RECOMMEND_A_BOOK_API_PROD_BASE_URL || '';
  }
  // In development, use empty string to rely on Vite's proxy
  return '';
};

// Create and export the API configuration
const apiConfig: ApiConfig = {
  baseURL: getBaseUrl(),
  endpoints: {
    recommendations: '/api/recommendations',
  },
  requestTimeout: 30000,
} as const;

export default apiConfig;
