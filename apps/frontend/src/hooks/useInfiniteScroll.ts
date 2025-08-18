import { useState, useEffect, useCallback } from 'react';

interface UseInfiniteScrollOptions {
  initialItemsToShow?: number;
  itemsToLoadPerPage?: number;
  threshold?: number;
}

interface UseInfiniteScrollResult<T> {
  visibleItems: T[];
  allItems: T[];
  setAllItems: (items: T[]) => void;
  hasMore: boolean;
  isLoading: boolean;
  resetScroll: () => void;
  searchPerformed: boolean;
}

export function useInfiniteScroll<T>({
  initialItemsToShow = 10,
  itemsToLoadPerPage = 10,
  threshold = 100,
}: UseInfiniteScrollOptions = {}): UseInfiniteScrollResult<T> {
  const [allItems, setAllItems] = useState<T[]>([]);
  const [visibleItems, setVisibleItems] = useState<T[]>([]);
  const [visibleCount, setVisibleCount] = useState(initialItemsToShow);
  const [isLoading, setIsLoading] = useState(false);
  const [searchPerformed, setSearchPerformed] = useState(false);

  const updateVisibleItems = useCallback(() => {
    setVisibleItems(allItems.slice(0, visibleCount));
  }, [allItems, visibleCount]);

  useEffect(() => {
    updateVisibleItems();
  }, [updateVisibleItems]);

  const handleScroll = useCallback(() => {
    if (isLoading) return;

    const scrollTop = window.scrollY || document.documentElement.scrollTop;
    const windowHeight = window.innerHeight;
    const documentHeight = document.documentElement.scrollHeight;

    // Check if user has scrolled to the threshold
    if (scrollTop + windowHeight >= documentHeight - threshold) {
      if (visibleCount < allItems.length) {
        setIsLoading(true);
        // Use setTimeout to simulate loading and prevent rapid multiple loads
        setTimeout(() => {
          setVisibleCount((prev) => Math.min(prev + itemsToLoadPerPage, allItems.length));
          setIsLoading(false);
        }, 300);
      }
    }
  }, [isLoading, visibleCount, allItems.length, threshold, itemsToLoadPerPage]);

  useEffect(() => {
    window.addEventListener('scroll', handleScroll);
    return () => window.removeEventListener('scroll', handleScroll);
  }, [handleScroll]);

  const resetScroll = useCallback(() => {
    setVisibleCount(initialItemsToShow);
    setAllItems([]);
    setVisibleItems([]);
    setIsLoading(false);
    setSearchPerformed(true);
  }, [initialItemsToShow]);

  const hasMore = visibleCount < allItems.length;

  return {
    visibleItems,
    allItems,
    setAllItems,
    hasMore,
    isLoading,
    resetScroll,
    searchPerformed,
  };
}
