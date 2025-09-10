/**
 * Formats a number into a string with a rating count.
 *
 * @param num - The number to format.
 * @returns A string representation of the formatted number with a rating count.
 */
export const formatRatingsCount = (num: number): string => {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1).replace(/\.0$/, '') + 'M';
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
  }
  return num.toString();
};

/**
 * Returns an array of book store links.
 *
 * @param isbn - The ISBN of the book.
 * @returns An array of book store links.
 */
export const getBookStoreLinks = (isbn?: string) => [
  { name: 'Hugendubel', url: `https://www.hugendubel.de` },
  {
    name: 'Thalia',
    url: isbn ? `https://www.thalia.de/suche?sq=${isbn}` : 'https://www.thalia.de',
  },
];
