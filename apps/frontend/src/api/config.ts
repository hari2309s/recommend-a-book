// Environment variable validation
const getApiBaseUrl = (): string => {
  const baseUrl = import.meta.env.VITE_RECOMMEND_A_BOOK_API_BASE_URL;
  const prodBaseUrl = import.meta.env.VITE_RECOMMEND_A_BOOK_API_PROD_BASE_URL;

  return import.meta.env.PROD ? prodBaseUrl : baseUrl;
};

// API configuration object
export const apiConfig = {
  baseURL: getApiBaseUrl(),
  endpoints: {
    recommendations: '/recommendations/',
  },
  requestTimeout: 30000, // 30 seconds
} as const;

export default apiConfig;
