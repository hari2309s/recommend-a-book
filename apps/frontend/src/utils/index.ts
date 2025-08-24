export const formatRatingsCount = (num: number): string => {
  if (num >= 1000000) {
    return (num / 1000000).toFixed(1).replace(/\.0$/, '') + 'M';
  }
  if (num >= 1000) {
    return (num / 1000).toFixed(1).replace(/\.0$/, '') + 'k';
  }
  return num.toString();
};

export const getBookStoreLinks = () => [
  { name: 'Hugendubel', url: `https://www.hugendubel.de` },
  { name: 'Thalia', url: `https://www.thalia.de` },
];
