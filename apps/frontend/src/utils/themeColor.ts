export const updateBrowserThemeColor = () => {
  // Get the computed CSS custom properties from Radix UI
  const root = document.documentElement;
  const computedStyle = getComputedStyle(root);

  // Try to get the Radix UI accent color
  const accentColor =
    computedStyle.getPropertyValue('--accent-9') ||
    computedStyle.getPropertyValue('--green-9') ||
    '#22c55e'; // fallback

  // Update or create theme-color meta tag
  let themeColorMeta = document.querySelector('meta[name="theme-color"]');
  if (!themeColorMeta) {
    themeColorMeta = document.createElement('meta');
    themeColorMeta.setAttribute('name', 'theme-color');
    document.head.appendChild(themeColorMeta);
  }

  // Set the color (convert CSS color to hex if needed)
  themeColorMeta.setAttribute('content', accentColor.trim() || '#22c55e');

  // Also update other browser-specific meta tags
  const msTileColor = document.querySelector('meta[name="msapplication-TileColor"]');
  if (msTileColor) {
    msTileColor.setAttribute('content', accentColor.trim() || '#22c55e');
  }

  const msNavButton = document.querySelector('meta[name="msapplication-navbutton-color"]');
  if (msNavButton) {
    msNavButton.setAttribute('content', accentColor.trim() || '#22c55e');
  }
};

// Call the function when the page loads
document.addEventListener('DOMContentLoaded', updateBrowserThemeColor);

// Also call it when the theme might change (if you support theme switching)
window.addEventListener('load', updateBrowserThemeColor);

// Optional: Listen for theme changes if you implement theme switching
const observer = new MutationObserver((mutations) => {
  mutations.forEach((mutation) => {
    if (
      mutation.type === 'attributes' &&
      (mutation.attributeName === 'class' || mutation.attributeName === 'data-theme')
    ) {
      setTimeout(updateBrowserThemeColor, 100); // Small delay to let CSS load
    }
  });
});

observer.observe(document.documentElement, {
  attributes: true,
  attributeFilter: ['class', 'data-theme'],
});
