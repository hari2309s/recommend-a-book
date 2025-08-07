export type PineconeRecord = {
  id: string;
  values: number[];
  metadata: {
    title: string;
    author: string;
    normalizedAuthor?: string; // For better searching
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
  rating: number | string; // Allow both for backward compatibility
  thumbnail: string;
  categories: string;
  publishedYear: number | string; // Allow both for backward compatibility
  ratingsCount: number | string; // Allow both for backward compatibility
  isbn13?: string; // Optional since some books might not have ISBN
}

export interface SearchHistory {
  id?: string;
  user_id: string;
  query: string;
  recommendations: Book[];
  created_at?: string;
}

// New interfaces for enhanced search functionality
export interface QueryIntent {
  type: 'author' | 'genre' | 'topic' | 'similar_to' | 'general';
  value: string;
  originalQuery: string;
  confidence?: number;
}

export interface SearchStrategy {
  useMetadataFilter: boolean;
  metadataField?: keyof Book;
  metadataValue?: string;
  semanticWeight: number;
  hybridSearch: boolean;
}

export interface SearchResult {
  id: string;
  score: number;
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
}
