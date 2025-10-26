import { useState, useEffect, useRef, type RefObject } from 'react';

type UseDynamicPaddingReturnType = {
  paddingTop: string;
  formRef: RefObject<HTMLDivElement | null>;
  updatePadding: () => void;
};

/**
 * Custom hook to dynamically calculate padding based on the height of a form element.
 * @returns {UseDynamicPaddingReturnType} An object containing the calculated padding,
 * a reference to the form element, and a function to update the padding.
 */
export const useDynamicPadding = (): UseDynamicPaddingReturnType => {
  const [paddingTop, setPaddingTop] = useState<string>('150px');
  const formRef = useRef<HTMLDivElement | null>(null);

  const updatePadding = () => {
    if (formRef.current) {
      requestAnimationFrame(() => {
        const formHeight = formRef.current?.offsetHeight;
        if (formHeight) {
          setPaddingTop(`${formHeight + 20}px`);
        }
      });
    }
  };

  useEffect(() => {
    updatePadding();
    window.addEventListener('resize', updatePadding);

    return () => window.removeEventListener('resize', updatePadding);
  }, []);

  return { paddingTop, formRef, updatePadding };
};
