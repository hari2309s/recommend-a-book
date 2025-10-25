import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@/index.css';
import App from '@/App.tsx';
import { Theme } from '@radix-ui/themes';
import { Toaster } from 'sonner';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Theme
      accentColor="brown"
      grayColor="olive"
      radius="medium"
      scaling="95%"
      className="custom-theme"
    >
      <App />
      <Toaster
        position="top-right"
        expand={false}
        richColors
        closeButton
        duration={5000}
        toastOptions={{
          style: {
            background: 'rgba(245, 245, 220, 0.95)',
            backdropFilter: 'blur(10px)',
            border: '1px solid rgba(139, 69, 19, 0.2)',
          },
        }}
      />
    </Theme>
  </StrictMode>
);
