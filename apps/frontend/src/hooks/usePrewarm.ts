import { useState, useEffect, useCallback, useRef } from 'react';
import apiConfig from '@api/config';

// Enhanced configuration for prewarming
const PREWARM_CONFIG = {
  // Endpoints to try in order (first one is preferred)
  ENDPOINTS: ['/prewarm', '/health'],
  // Maximum number of retry attempts
  MAX_RETRIES: 3,
  // Base delay between retries in ms (will use exponential backoff)
  RETRY_DELAY_MS: 2000,
  // Timeout for each prewarm request in ms - increased for cold starts
  TIMEOUT_MS: 45000, // 45 seconds to handle cold starts
  // Whether to log prewarm activities to console (disable in production)
  ENABLE_LOGGING: import.meta.env.DEV || import.meta.env.VITE_ENABLE_PREWARM_LOGS === 'true',
  // Cache duration for successful prewarm (in ms)
  CACHE_DURATION_MS: 15 * 60 * 1000, // 15 minutes
};

// Prewarm status enum
export enum PrewarmStatus {
  NOT_STARTED = 'not_started',
  IN_PROGRESS = 'in_progress',
  SUCCESS = 'success',
  PARTIAL_SUCCESS = 'partial_success',
  FAILED = 'failed',
}

// Prewarm state interface
interface PrewarmState {
  status: PrewarmStatus;
  lastAttempt: number;
  isInitialLoad: boolean;
  error?: string;
}

// Hook return type
interface UsePrewarmReturn {
  status: PrewarmStatus;
  isPrewarmed: boolean;
  prewarmApi: (force?: boolean) => Promise<PrewarmStatus>;
  isPrewarming: boolean;
  error?: string;
}

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
 */
async function pingEndpoint(
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
      url = normalizedEndpoint.startsWith('/api/')
        ? normalizedEndpoint
        : `/api${normalizedEndpoint}`;
    } else {
      // In production, use the relative path from apiConfig
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
          // Exponential backoff with jitter
          const baseDelay = PREWARM_CONFIG.RETRY_DELAY_MS * Math.pow(2, attempt - 1);
          const jitter = Math.random() * 1000; // Add up to 1 second of jitter
          const delay = baseDelay + jitter;
          logPrewarm(`Retry attempt ${attempt}/${retries} after ${Math.round(delay)}ms delay`);
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
 * Custom hook for API prewarming functionality
 */
export function usePrewarm(): UsePrewarmReturn {
  const [state, setState] = useState<PrewarmState>({
    status: PrewarmStatus.NOT_STARTED,
    lastAttempt: 0,
    isInitialLoad: true,
  });

  const [isPrewarming, setIsPrewarming] = useState(false);
  const abortControllerRef = useRef<AbortController | null>(null);

  // Check if API is prewarmed
  const isPrewarmed = state.status === PrewarmStatus.SUCCESS && 
    (Date.now() - state.lastAttempt) < PREWARM_CONFIG.CACHE_DURATION_MS;

  /**
   * Attempts to prewarm the API by trying multiple endpoints in sequence
   */
  const prewarmApi = useCallback(async (force: boolean = false): Promise<PrewarmStatus> => {
    const now = Date.now();

    // Skip if already prewarmed recently unless forced
    if (
      !force &&
      state.status === PrewarmStatus.SUCCESS &&
      now - state.lastAttempt < PREWARM_CONFIG.CACHE_DURATION_MS
    ) {
      logPrewarm('API already prewarmed recently, skipping');
      return PrewarmStatus.SUCCESS;
    }

    // Cancel any existing prewarm operation
    if (abortControllerRef.current) {
      abortControllerRef.current.abort();
    }

    // Create new abort controller
    abortControllerRef.current = new AbortController();

    setIsPrewarming(true);
    setState(prev => ({
      ...prev,
      status: PrewarmStatus.IN_PROGRESS,
      lastAttempt: now,
      isInitialLoad: false,
      error: undefined,
    }));

    logPrewarm('Starting API prewarm sequence');

    try {
      // Try each endpoint in order until one succeeds
      let anySuccess = false;

      for (const endpoint of PREWARM_CONFIG.ENDPOINTS) {
        if (abortControllerRef.current?.signal.aborted) {
          logPrewarm('Prewarm operation aborted by caller', 'warn');
          setState(prev => ({ ...prev, status: PrewarmStatus.FAILED }));
          return PrewarmStatus.FAILED;
        }

        const success = await pingEndpoint(endpoint, { 
          signal: abortControllerRef.current?.signal 
        });

        if (success) {
          anySuccess = true;
          break;
        }
      }

      // Update state based on result
      if (anySuccess) {
        setState(prev => ({ ...prev, status: PrewarmStatus.SUCCESS }));
        logPrewarm('API prewarm completed successfully');
        return PrewarmStatus.SUCCESS;
      } else {
        setState(prev => ({ 
          ...prev, 
          status: PrewarmStatus.FAILED,
          error: 'All prewarm attempts failed'
        }));
        logPrewarm('All prewarm attempts failed', 'error');
        return PrewarmStatus.FAILED;
      }
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : 'Unknown error';
      setState(prev => ({ 
        ...prev, 
        status: PrewarmStatus.FAILED,
        error: errorMessage
      }));
      logPrewarm(`Prewarm error: ${errorMessage}`, 'error');
      return PrewarmStatus.FAILED;
    } finally {
      setIsPrewarming(false);
      abortControllerRef.current = null;
    }
  }, [state.status, state.lastAttempt]);

  /**
   * Auto-prewarm on initial load
   */
  useEffect(() => {
    if (state.isInitialLoad) {
      // Delay initial prewarm to not compete with critical resources
      const timeoutId = setTimeout(() => {
        logPrewarm('Auto-prewarming API on initial page load');
        prewarmApi().catch((err) => {
          logPrewarm(`Error during initial prewarm: ${err.message}`, 'error');
        });
      }, 1000); // Reduced delay for faster initial prewarm

      return () => clearTimeout(timeoutId);
    }
  }, [state.isInitialLoad, prewarmApi]);

  /**
   * Prewarm when page becomes visible after being hidden
   */
  useEffect(() => {
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        const now = Date.now();
        // Only prewarm if it's been at least 10 minutes since last prewarm
        if (now - state.lastAttempt > 10 * 60 * 1000) {
          logPrewarm('Prewarming API after tab became visible');
          prewarmApi().catch((err) => {
            logPrewarm(`Error during visibility prewarm: ${err.message}`, 'error');
          });
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, [state.lastAttempt, prewarmApi]);

  /**
   * Cleanup on unmount
   */
  useEffect(() => {
    return () => {
      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }
    };
  }, []);

  return {
    status: state.status,
    isPrewarmed,
    prewarmApi,
    isPrewarming,
    error: state.error,
  };
}

export default usePrewarm;
