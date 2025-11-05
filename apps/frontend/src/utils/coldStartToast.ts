import { toast } from 'sonner';
import { TOAST_MESSAGES, formatRetryMessage } from '@/utils';

const COLD_START_DELAY_MS = 5000; // Show toast only if request takes > 5 seconds (cold start indicator)
const IS_PRODUCTION = import.meta.env.PROD;

class ColdStartToastManager {
  private toastId: string | number | null = null;
  private timerId: number | null = null;
  private requestStartTime: number | null = null;
  private hasShownToastInSession = false;

  /**
   * Mark the start of a request and schedule toast to show after delay
   */
  start(): void {
    // In development, don't show cold start toasts as the API is always warm
    if (!IS_PRODUCTION) {
      return;
    }

    this.requestStartTime = Date.now();
    this.clearTimer();

    // Schedule toast to show after delay if request is still in progress
    this.timerId = window.setTimeout(() => {
      if (this.requestStartTime !== null && this.toastId === null) {
        this.showToast();
      }
    }, COLD_START_DELAY_MS);
  }

  /**
   * Show the cold start toast
   */
  private showToast(): void {
    if (this.toastId !== null) return;

    // Only show the detailed cold start message on the first occurrence
    const description = !this.hasShownToastInSession
      ? TOAST_MESSAGES.COLD_START_DESCRIPTION
      : TOAST_MESSAGES.COLD_START_RETRY_DESCRIPTION;

    this.toastId = toast.loading(TOAST_MESSAGES.COLD_START_LOADING, {
      description,
      duration: Infinity,
    });

    this.hasShownToastInSession = true;
  }

  /**
   * Update toast message for retry attempts
   */
  retry(attempt: number, maxRetries: number): void {
    // Only handle retries in production
    if (!IS_PRODUCTION) {
      return;
    }

    if (this.toastId !== null) {
      toast.loading(formatRetryMessage(attempt, maxRetries), {
        id: this.toastId,
        description: TOAST_MESSAGES.COLD_START_RETRY_DESCRIPTION,
      });
    }
  }

  /**
   * Dismiss the toast (on success or error)
   */
  dismiss(): void {
    this.clearTimer();

    if (this.toastId !== null) {
      toast.dismiss(this.toastId);
      this.toastId = null;
    }

    this.requestStartTime = null;
  }

  /**
   * Clear the scheduled timer
   */
  private clearTimer(): void {
    if (this.timerId !== null) {
      window.clearTimeout(this.timerId);
      this.timerId = null;
    }
  }

  /**
   * Cleanup all resources
   */
  cleanup(): void {
    this.dismiss();
  }

  /**
   * Reset session state
   */
  resetSession(): void {
    this.hasShownToastInSession = false;
  }
}

export const coldStartToastManager = new ColdStartToastManager();
