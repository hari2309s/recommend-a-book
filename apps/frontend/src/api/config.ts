/**
 * API Configuration
 *
 * Simple configuration for API endpoints in both development and production.
 */

// API configuration object
const apiConfig = {
  // Base URL from environment variables (already includes /api)
  baseURL: import.meta.env.PROD
    ? import.meta.env.VITE_RECOMMEND_A_BOOK_API_PROD_BASE_URL || ''
    : import.meta.env.VITE_RECOMMEND_A_BOOK_API_BASE_URL || '',

  // Endpoints (without /api prefix since it's already in the base URL)
  endpoints: {
    recommendations: '/recommendations',
  },

  // Request timeout in milliseconds
  requestTimeout: 30000,
};

export default apiConfig;
