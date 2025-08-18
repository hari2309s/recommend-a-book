import { z } from 'zod';

// Environment variables schema
const envSchema = z.object({
  VITE_API_URL: z.string().url().default('http://localhost:3000/api'),
  VITE_PROD_API_URL: z.string().url(),
  VITE_ENABLE_MOCK_API: z
    .string()
    .transform((v) => v === 'true')
    .default(false),
  VITE_ENABLE_ANALYTICS: z
    .string()
    .transform((v) => v === 'true')
    .default(false),
});

// Validate environment variables
const env = envSchema.parse(import.meta.env);

// API version
// Determine the base URL based on environment
const BASE_URL = import.meta.env.MODE === 'production' ? env.VITE_PROD_API_URL : env.VITE_API_URL;

// API configuration object
export const apiConfig = {
  baseURL: BASE_URL,
  enableMockApi: env.VITE_ENABLE_MOCK_API,
  enableAnalytics: env.VITE_ENABLE_ANALYTICS,
  endpoints: {
    // Health check
    health: '/health',

    // Books
    books: '/books',

    // Recommendations
    recommendations: '/recommendations',

    // Search history
    searchHistory: '/search-history',

    // User preferences (for future use)
    preferences: '/preferences',
  },
  // Request timeouts (in milliseconds)
  timeouts: {
    default: 30000, // 30 seconds
    recommendations: 45000, // 45 seconds for recommendations
    upload: 60000, // 60 seconds for uploads
  },
  // Rate limiting
  rateLimit: {
    maxRequestsPerMinute: 100,
    recommendationsPerMinute: 20,
  },
  // Pagination defaults
  pagination: {
    defaultPage: 1,
    defaultPerPage: 20,
    maxPerPage: 100,
  },
} as const;

// Type for the entire config
export type ApiConfig = typeof apiConfig;

// Export some useful types
export type ApiEndpoint = keyof typeof apiConfig.endpoints;
export type ApiTimeout = keyof typeof apiConfig.timeouts;

export default apiConfig;
