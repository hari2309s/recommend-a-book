/** @type {import('tailwindcss').Config} */
export default {
  content: ["./src/**/*.{js,jsx,ts,tsx}"],
  theme: {
    extend: {
      colors: {
        fiction: {
          primary: '#1E3A8A', // Deep Blue
          secondary: '#D1D5DB', // Soft Gray
        },
        mystery: {
          primary: '#1C2526', // Charcoal Black
          secondary: '#DC2626', // Crimson Red
        },
        'science-fiction': {
          primary: '#00B7EB', // Neon Blue
          secondary: '#A1A1AA', // Metallic Silver
        },
        fantasy: {
          primary: '#14532D', // Forest Green
          secondary: '#FBBF24', // Gold
        },
        romance: {
          primary: '#F472B6', // Blush Pink
          secondary: '#F87171', // Warm Coral
        },
        horror: {
          primary: '#191970', // Midnight Blue
          secondary: '#FF4500', // Blood Orange
        },
        'historical-fiction': {
          primary: '#8B4513', // Sepia Brown
          secondary: '#D4A017', // Antique Gold
        },
        nonfiction: {
          primary: '#475569', // Slate Blue
          secondary: '#F5F5F5', // Cream
        },
        biography: {
          primary: '#D2B48C', // Warm Beige
          secondary: '#2E8B57', // Soft Teal
        },
        'young-adult': {
          primary: '#9333EA', // Bright Purple
          secondary: '#FACC15', // Sunny Yellow
        },
        children: {
          primary: '#FF69B4', // Bubblegum Pink
          secondary: '#87CEEB', // Sky Blue
        },
        'science-tech': {
          primary: '#4682B4', // Steel Blue
          secondary: '#FFFFFF', // Bright White
        },
        'self-help': {
          primary: '#FFD700', // Sunshine Yellow
          secondary: '#90EE90', // Soft Green
        },
        poetry: {
          primary: '#E6E6FA', // Lavender
          secondary: '#4B0082', // Deep Violet
        },
        'graphic-novels': {
          primary: '#EF4444', // Vibrant Red
          secondary: '#3B82F6', // Electric Blue
        },
      },
    },
  },
  plugins: [],
}
