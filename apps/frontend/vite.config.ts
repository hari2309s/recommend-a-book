import { defineConfig, loadEnv } from 'vite';
import react from '@vitejs/plugin-react';
import path from 'path';

// https://vitejs.dev/config/
export default defineConfig(({ mode }) => {
  // Load env file based on `mode` in the current working directory.
  // Set the third parameter to '' to load all env regardless of the `VITE_` prefix.
  const env = loadEnv(mode, process.cwd(), '');

  return {
    plugins: [react()],

    resolve: {
      alias: {
        '@': path.resolve(__dirname, './src'),
        '@components': path.resolve(__dirname, './src/components'),
        '@hooks': path.resolve(__dirname, './src/hooks'),
        '@assets': path.resolve(__dirname, './src/assets'),
        '@api': path.resolve(__dirname, './src/api'),
      },
    },

    // Server options
    server: {
      port: 3000,
      // Configure proxy for local development
      proxy: {
        // Forward /api requests to the Rust backend
        '/api': {
          target: env.VITE_API_URL || 'http://localhost:10000',
          changeOrigin: true,
          secure: false,
          ws: true,
          // Rewrite path if needed
          // rewrite: (path) => path.replace(/^\/api/, '')
        },
      },
    },

    // Build options
    build: {
      // Output directory
      outDir: 'dist',
      // Generate sourcemaps for production build
      sourcemap: true,
      // Configure rollup options
      rollupOptions: {
        output: {
          manualChunks: {
            react: ['react', 'react-dom'],
            // Add other vendor chunks as needed
          },
        },
      },
    },

    // Optimizations
    optimizeDeps: {
      include: ['react', 'react-dom'],
    },

    // Environment variables
    envPrefix: 'VITE_',

    // CSS configuration
    css: {
      devSourcemap: true,
      modules: {
        localsConvention: 'camelCase',
      },
    },
  };
});
