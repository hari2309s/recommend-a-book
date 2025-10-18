import { useState, useEffect, useCallback, useRef } from 'react';
import apiConfig from '@api/config';

const PREWARM_CONFIG = {
  ENDPOINTS: ['/prewarm', '/health'],
  MAX_RETRIES: 3,
  RETRY_DELAY_MS: 2000,
  TIMEOUT_MS: 45000,
  ENABLE_LOGGING: import.meta.env.DEV || import.meta.env.VITE_ENABLE_PREWARM_LOGS === 'true',
  CACHE_DURATION_MS: 15 * 60 * 1000,
} as const;

export enum PrewarmStatus {
  NOT_STARTED = 'not_started',
  IN_PROGRESS = 'in_progress',
  SUCCESS = 'success',
  PARTIAL_SUCCESS = 'partial_success',
  FAILED = 'failed',
}

interface PrewarmState {
  status: PrewarmStatus;
  lastAttempt: number;
  isInitialLoad: boolean;
  error?: string;
}

interface UsePrewarmReturn {
  status: PrewarmStatus;
  isPrewarmed: boolean;
  prewarmApi: (force?: boolean) => Promise<PrewarmStatus>;
  isPrewarming: boolean;
  error?: string;
}

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

  const timeoutId = setTimeout(() => controller.abort(), PREWARM_CONFIG.TIMEOUT_MS);

  try {
    const normalizedEndpoint = endpoint.startsWith('/') ? endpoint : `/${endpoint}`;
    let url: string;
    if (import.meta.env.DEV) {
      url = normalizedEndpoint.startsWith('/api/')
        ? normalizedEndpoint
        : `/api${normalizedEndpoint}`;
    } else {
      url = `${apiConfig.baseURL}${normalizedEndpoint}`;
    }

    logPrewarm(`Pinging ${url}...`);

    for (let attempt = 0; attempt <= retries; attempt++) {
      if (combinedSignal.aborted) {
        throw new Error('Request aborted');
      }

      try {
        if (attempt > 0) {
          const baseDelay = PREWARM_CONFIG.RETRY_DELAY_MS * Math.pow(2, attempt - 1);
          const jitter = Math.random() * 1000;
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
          break;
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

export function usePrewarm(): UsePrewarmReturn {
  const [state, setState] = useState<PrewarmState>({
    status: PrewarmStatus.NOT_STARTED,
    lastAttempt: 0,
    isInitialLoad: true,
  });

  const [isPrewarming, setIsPrewarming] = useState<boolean>(false);
  const abortControllerRef = useRef<AbortController | null>(null);

  const isPrewarmed: boolean =
    state.status === PrewarmStatus.SUCCESS &&
    Date.now() - state.lastAttempt < PREWARM_CONFIG.CACHE_DURATION_MS;

  const prewarmApi = useCallback(
    async (force: boolean = false): Promise<PrewarmStatus> => {
      const now = Date.now();

      if (
        !force &&
        state.status === PrewarmStatus.SUCCESS &&
        now - state.lastAttempt < PREWARM_CONFIG.CACHE_DURATION_MS
      ) {
        logPrewarm('API already prewarmed recently, skipping');
        return PrewarmStatus.SUCCESS;
      }

      if (abortControllerRef.current) {
        abortControllerRef.current.abort();
      }

      abortControllerRef.current = new AbortController();

      setIsPrewarming(true);
      setState((prev) => ({
        ...prev,
        status: PrewarmStatus.IN_PROGRESS,
        lastAttempt: now,
        isInitialLoad: false,
        error: undefined,
      }));

      logPrewarm('Starting API prewarm sequence');

      try {
        let anySuccess = false;

        for (const endpoint of PREWARM_CONFIG.ENDPOINTS) {
          if (abortControllerRef.current?.signal.aborted) {
            logPrewarm('Prewarm operation aborted by caller', 'warn');
            setState((prev) => ({ ...prev, status: PrewarmStatus.FAILED }));
            return PrewarmStatus.FAILED;
          }

          const success = await pingEndpoint(endpoint, {
            signal: abortControllerRef.current?.signal,
          });

          if (success) {
            anySuccess = true;
            break;
          }
        }

        if (anySuccess) {
          setState((prev) => ({ ...prev, status: PrewarmStatus.SUCCESS }));
          logPrewarm('API prewarm completed successfully');
          return PrewarmStatus.SUCCESS;
        } else {
          setState((prev) => ({
            ...prev,
            status: PrewarmStatus.FAILED,
            error: 'All prewarm attempts failed',
          }));
          logPrewarm('All prewarm attempts failed', 'error');
          return PrewarmStatus.FAILED;
        }
      } catch (error) {
        const errorMessage = error instanceof Error ? error.message : 'Unknown error';
        setState((prev) => ({
          ...prev,
          status: PrewarmStatus.FAILED,
          error: errorMessage,
        }));
        logPrewarm(`Prewarm error: ${errorMessage}`, 'error');
        return PrewarmStatus.FAILED;
      } finally {
        setIsPrewarming(false);
        abortControllerRef.current = null;
      }
    },
    [state.status, state.lastAttempt]
  );

  // Auto-prewarm on initial load (silently in background)
  useEffect(() => {
    if (state.isInitialLoad) {
      const timeoutId = setTimeout(() => {
        logPrewarm('Auto-prewarming API on initial page load');
        prewarmApi().catch((err) => {
          logPrewarm(
            `Error during initial prewarm: ${err instanceof Error ? err.message : 'Unknown error'}`,
            'error'
          );
        });
      }, 1000);

      return () => clearTimeout(timeoutId);
    }
  }, [state.isInitialLoad, prewarmApi]);

  // Prewarm when page becomes visible after being hidden
  useEffect(() => {
    const handleVisibilityChange = (): void => {
      if (document.visibilityState === 'visible') {
        const now = Date.now();
        if (now - state.lastAttempt > 10 * 60 * 1000) {
          logPrewarm('Prewarming API after tab became visible');
          prewarmApi().catch((err) => {
            logPrewarm(
              `Error during visibility prewarm: ${err instanceof Error ? err.message : 'Unknown error'}`,
              'error'
            );
          });
        }
      }
    };

    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () => document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, [state.lastAttempt, prewarmApi]);

  // Cleanup on unmount
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
