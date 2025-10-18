/**
 * API Configuration
 *
 * Simple configuration for API endpoints in both development and production.
 */

// API configuration object
const apiConfig = {
  // Base URL from environment variables (already includes /api)
  baseURL: import.meta.env.PROD
    ? import.meta.env.VITE_RECOMMEND_A_BOOK_API_PROD_BASE_URL ||
      'https://recommend-a-book-api.onrender.com/api'
    : import.meta.env.VITE_RECOMMEND_A_BOOK_API_BASE_URL || 'http://127.0.0.1:10000/api',

  // Endpoints (without /api prefix since it's already in the base URL)
  endpoints: {
    recommendations: '/recommendations',
    health: '/health',
    prewarm: '/prewarm',
  },

  // Request timeout in milliseconds - increased for cold starts
  requestTimeout: 60000, // 60 seconds to handle cold starts
};

export default apiConfig;
