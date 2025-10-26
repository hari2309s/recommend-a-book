import { toast } from 'sonner';
import { TOAST_MESSAGES, formatRetryMessage } from '@/utils';

const COLD_START_DELAY_MS = 2000; // Show toast only if request takes > 2 seconds

class ColdStartToastManager {
  private toastId: string | number | null = null;
  private timerId: number | null = null;
  private requestStartTime: number | null = null;

  /**
   * Mark the start of a request and schedule toast to show after delay
   */
  start(): void {
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

    this.toastId = toast.loading(TOAST_MESSAGES.COLD_START_LOADING, {
      description: TOAST_MESSAGES.COLD_START_DESCRIPTION,
      duration: Infinity,
    });
  }

  /**
   * Update toast message for retry attempts
   */
  retry(attempt: number, maxRetries: number): void {
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
}

export const coldStartToastManager = new ColdStartToastManager();
