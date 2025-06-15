export type PineconeRecord = {
  id: string;
  values: number[];
  metadata: {
    title: string;
    author: string;
    description: string;
    rating: number;
    thumbnail: string;
    categories: string;
    publishedYear: number;
    ratingsCount: number;
  };
};

export interface Book {
  title: string;
  author: string;
  description: string;
  rating: number;
  thumbnail: string;
  categories: string;
  publishedYear: number;
  ratingsCount: number;
  isbn13: string;
}
