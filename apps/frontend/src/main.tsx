import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@/index.css';
import App from '@/App.tsx';
import { Theme } from '@radix-ui/themes';
import { updateBrowserThemeColor } from '@/utils/themeColor';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Theme accentColor="green" grayColor="slate" radius="medium" scaling="95%">
      <App />
    </Theme>
  </StrictMode>
);

updateBrowserThemeColor();
