export const APP_MESSAGES = {
  TITLE: 'Book Recommendation System',
  LOGO_ALT: 'Book Recommendation System',
  LOADING: 'Fetching books...',
  NO_RECOMMENDATIONS: 'No books found',
  ERROR_LOADING: 'Error loading books',
  INFO: 'Try searching for a book.',
};

export const SEARCH_MESSAGES = {
  PLACEHOLDER: 'Enter your book preferences',
  SUBMIT_LABEL: 'Get Recommendations',
  DETECTED_THEMES_LABEL: 'Detected themes:',
};

export const TOAST_MESSAGES = {
  INVALID_RESPONSE: 'Invalid response format',
  INVALID_RESPONSE_DESCRIPTION: 'Please try again.',
  FETCH_FAILED: 'Failed to fetch recommendations',
  FETCH_FAILED_DESCRIPTION: 'Please try again later.',
  TIMEOUT: 'Request timed out',
  TIMEOUT_DESCRIPTION: 'The API is taking longer than expected. Please try again.',
  NETWORK_ERROR: 'Connection failed',
  NETWORK_ERROR_DESCRIPTION: 'Could not reach the API server. Check your connection.',
  COLD_START_LOADING: '🔥 Warming up the API...',
  COLD_START_DESCRIPTION:
    'First request detected. The API is starting up. This will be faster next time!',
  COLD_START_RETRY_DESCRIPTION: 'Still warming up. Please wait...',
};

export const formatRetryMessage = (attempt: number, maxRetries: number): string =>
  `Retry ${attempt}/${maxRetries}...`;

export const BOOK_DESCRIPTION_MESSAGES = {
  DESCRIPTION_LABEL: 'Description',
};

export const IMAGE_ALT_MESSAGES = {
  ERROR: 'Error',
  INFO: 'Info',
};
