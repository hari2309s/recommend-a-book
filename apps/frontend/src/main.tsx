import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@/index.css';
import App from '@/App.tsx';
import { Theme } from '@radix-ui/themes';
import { initializePrewarm } from '@/utils';
import { Toaster } from 'sonner';

// Initialize API prewarming to prevent cold starts
initializePrewarm();

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Theme accentColor="green" grayColor="slate" radius="medium" scaling="95%">
      <App />
      <Toaster
        position="top-right"
        expand={false}
        richColors
        closeButton
        duration={5000}
        toastOptions={{
          style: {
            background: 'rgba(255, 255, 255, 0.95)',
            backdropFilter: 'blur(10px)',
            border: '1px solid rgba(34, 197, 94, 0.2)',
          },
        }}
      />
    </Theme>
  </StrictMode>
);
