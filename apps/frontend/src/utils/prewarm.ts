import apiConfig from '@api/config';

/**
 * API Prewarming Utility
 *
 * This utility helps mitigate cold start issues by proactively
 * warming up the API when the frontend application loads.
 */

// Configuration for prewarming
const PREWARM_CONFIG = {
  // Endpoints to try in order (first one is preferred)
  ENDPOINTS: ['/prewarm', '/health'],
  // Maximum number of retry attempts
  MAX_RETRIES: 2,
  // Base delay between retries in ms (will use exponential backoff)
  RETRY_DELAY_MS: 1000,
  // Timeout for each prewarm request in ms
  TIMEOUT_MS: 20000,
  // Whether to log prewarm activities to console (disable in production)
  ENABLE_LOGGING: import.meta.env.DEV || import.meta.env.VITE_ENABLE_PREWARM_LOGS === 'true',
};

// Prewarm status
export enum PrewarmStatus {
  NOT_STARTED = 'not_started',
  IN_PROGRESS = 'in_progress',
  SUCCESS = 'success',
  PARTIAL_SUCCESS = 'partial_success',
  FAILED = 'failed',
}

// Current prewarm state
let prewarmState = {
  status: PrewarmStatus.NOT_STARTED,
  lastAttempt: 0,
  // Track whether this is initial page load
  isInitialLoad: true,
};

/**
 * Logs prewarm-related messages to console if logging is enabled
 */
function logPrewarm(message: string, level: 'info' | 'warn' | 'error' = 'info'): void {
  if (!PREWARM_CONFIG.ENABLE_LOGGING) return;

  const prefix = '[API Prewarm]';

  switch (level) {
    case 'info':
      console.info(`${prefix} ${message}`);
      break;
    case 'warn':
      console.warn(`${prefix} ${message}`);
      break;
    case 'error':
      console.error(`${prefix} ${message}`);
      break;
  }
}

/**
 * Pings a specific API endpoint with retry logic
 * @param endpoint - The endpoint to ping (e.g., '/prewarm')
 * @param options - Optional configuration
 * @returns Promise that resolves to true if successful, false otherwise
 */
export async function pingEndpoint(
  endpoint: string,
  options: {
    retries?: number;
    signal?: AbortSignal;
  } = {}
): Promise<boolean> {
  const { retries = PREWARM_CONFIG.MAX_RETRIES, signal } = options;
  const controller = new AbortController();
  const combinedSignal = signal
    ? ({ aborted: signal.aborted || controller.signal.aborted } as AbortSignal)
    : controller.signal;

  // Set timeout
  const timeoutId = setTimeout(() => controller.abort(), PREWARM_CONFIG.TIMEOUT_MS);

  try {
    // Make sure endpoint starts with /
    const normalizedEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;

    // Construct URL - use proxy in development, direct URLs in production
    let url;
    if (import.meta.env.DEV) {
      // In development, use relative URLs to leverage Vite proxy
      // The /api prefix should already be handled by the proxy config
      url = normalizedEndpoint.startsWith('/api/')
        ? normalizedEndpoint
        : `/api${normalizedEndpoint}`;
    } else {
      // In production, use the relative path from apiConfig
      // The base URL already includes /api
      url = `${apiConfig.baseURL}${normalizedEndpoint}`;
    }

    logPrewarm(`Pinging ${url}...`);

    // Try the request with retries
    for (let attempt = 0; attempt <= retries; attempt++) {
      if (combinedSignal.aborted) {
        throw new Error('Request aborted');
      }

      try {
        if (attempt > 0) {
          // Exponential backoff
          const delay = PREWARM_CONFIG.RETRY_DELAY_MS * Math.pow(2, attempt - 1);
          logPrewarm(`Retry attempt ${attempt}/${retries} after ${delay}ms delay`);
          await new Promise((resolve) => setTimeout(resolve, delay));
        }

        const response = await fetch(url, {
          method: 'GET',
          headers: {
            'Cache-Control': 'no-cache',
            Pragma: 'no-cache',
            'X-Prewarm-Source': 'frontend-client',
            'Content-Type': 'application/json',
          },
          mode: 'cors',
          credentials: 'omit',
          signal: combinedSignal,
        });

        if (response.ok) {
          logPrewarm(`Successfully prewarmed API using ${endpoint} endpoint`);
          return true;
        } else {
          logPrewarm(
            `Prewarm request to ${endpoint} failed with status ${response.status}`,
            'warn'
          );
        }
      } catch (err) {
        const error = err as Error;
        if (error.name === 'AbortError') {
          logPrewarm(
            `Prewarm request to ${endpoint} timed out after ${PREWARM_CONFIG.TIMEOUT_MS}ms`,
            'warn'
          );
          break; // Don't retry timeouts
        } else {
          logPrewarm(`Error during prewarm attempt ${attempt}: ${error.message}`, 'warn');
        }
      }
    }

    logPrewarm(`All retry attempts failed for endpoint ${endpoint}`, 'error');
    return false;
  } finally {
    clearTimeout(timeoutId);
  }
}

/**
 * Attempts to prewarm the API by trying multiple endpoints in sequence
 * @param options - Optional configuration
 * @returns Promise with prewarm status
 */
export async function prewarmApi(
  options: {
    force?: boolean;
    signal?: AbortSignal;
  } = {}
): Promise<PrewarmStatus> {
  const { force = false, signal } = options;
  const now = Date.now();

  // Skip if already prewarmed recently (within 5 minutes) unless forced
  if (
    !force &&
    prewarmState.status === PrewarmStatus.SUCCESS &&
    now - prewarmState.lastAttempt < 19 * 60 * 1000
  ) {
    logPrewarm('API already prewarmed recently, skipping');
    return PrewarmStatus.SUCCESS;
  }

  // Set state to in-progress
  prewarmState = {
    ...prewarmState,
    status: PrewarmStatus.IN_PROGRESS,
    lastAttempt: now,
    isInitialLoad: false,
  };

  logPrewarm('Starting API prewarm sequence');

  // Try each endpoint in order until one succeeds
  let anySuccess = false;

  for (const endpoint of PREWARM_CONFIG.ENDPOINTS) {
    if (signal?.aborted) {
      logPrewarm('Prewarm operation aborted by caller', 'warn');
      prewarmState.status = PrewarmStatus.FAILED;
      return PrewarmStatus.FAILED;
    }

    const success = await pingEndpoint(endpoint, { signal });

    if (success) {
      anySuccess = true;
      break;
    }
  }

  // Update state based on result
  if (anySuccess) {
    prewarmState.status = PrewarmStatus.SUCCESS;
    logPrewarm('API prewarm completed successfully');
    return PrewarmStatus.SUCCESS;
  } else {
    prewarmState.status = PrewarmStatus.FAILED;
    logPrewarm('All prewarm attempts failed', 'error');
    return PrewarmStatus.FAILED;
  }
}

/**
 * Gets the current prewarm status
 */
export function getPrewarmStatus(): PrewarmStatus {
  return prewarmState.status;
}

/**
 * Initialize prewarm system
 * - Automatically prewarms on initial page load
 * - Sets up visibility change listener to prewarm when tab becomes visible after being inactive
 */
export function initializePrewarm(): void {
  // Trigger prewarm after a short delay to not compete with critical resources
  setTimeout(() => {
    if (prewarmState.isInitialLoad) {
      logPrewarm('Auto-prewarming API on initial page load');
      prewarmApi().catch((err) => {
        logPrewarm(`Error during initial prewarm: ${err.message}`, 'error');
      });
    }
  }, 1500);

  // Also prewarm when the page becomes visible after being hidden
  // This helps when users return to the app after it's been in the background
  document.addEventListener('visibilitychange', () => {
    if (document.visibilityState === 'visible') {
      const now = Date.now();
      // Only prewarm if it's been at least 19 minutes since last prewarm
      if (now - prewarmState.lastAttempt > 19 * 60 * 1000) {
        logPrewarm('Prewarming API after tab became visible');
        prewarmApi().catch((err) => {
          logPrewarm(`Error during visibility prewarm: ${err.message}`, 'error');
        });
      }
    }
  });
}

// Export default object for easier imports
export default {
  prewarmApi,
  pingEndpoint,
  getPrewarmStatus,
  initializePrewarm,
  PrewarmStatus,
};
