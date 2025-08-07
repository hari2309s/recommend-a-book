import * as use from '@tensorflow-models/universal-sentence-encoder';
import * as tf from '@tensorflow/tfjs';
import { Book } from '@/types';

interface QueryIntent {
  type: 'author' | 'genre' | 'topic' | 'similar_to' | 'general';
  value: string;
  originalQuery: string;
}

interface SearchStrategy {
  useMetadataFilter: boolean;
  metadataField?: keyof Book;
  metadataValue?: string;
  semanticWeight: number;
  hybridSearch: boolean;
}

export class RecommendationService {
  private model: use.UniversalSentenceEncoder;
  private pineconeIndex: any;

  constructor(model: use.UniversalSentenceEncoder, pineconeIndex: any) {
    this.model = model;
    this.pineconeIndex = pineconeIndex;
  }

  /**
   * Parse the user query to understand intent
   */
  private parseQueryIntent(query: string): QueryIntent {
    const lowerQuery = query.toLowerCase().trim();

    // Author patterns
    const authorPatterns = [
      /(?:books?\s+)?(?:written\s+)?by\s+([a-zA-Z\s.'-]+)/i,
      /(?:works?\s+)?(?:of|from)\s+([a-zA-Z\s.'-]+)/i,
      /([a-zA-Z\s.'-]+)'s\s+books?/i,
      /author:?\s*([a-zA-Z\s.'-]+)/i,
    ];

    for (const pattern of authorPatterns) {
      const match = query.match(pattern);
      if (match) {
        const author = match[1].trim().replace(/\s+/g, ' ');
        return {
          type: 'author',
          value: author,
          originalQuery: query,
        };
      }
    }

    // Genre patterns
    const genrePatterns = [
      /(?:genre:?\s*)?(?:books?\s+in\s+)?([a-zA-Z\s&-]+?)\s+(?:books?|novels?|genre)/i,
      /(?:recommend\s+)?([a-zA-Z\s&-]+?)\s+(?:books?|novels?|fiction|non-fiction)/i,
    ];

    const commonGenres = [
      'fiction',
      'non-fiction',
      'mystery',
      'romance',
      'fantasy',
      'sci-fi',
      'science fiction',
      'biography',
      'history',
      'self-help',
      'business',
      'philosophy',
      'poetry',
      'drama',
      'thriller',
      'horror',
      'young adult',
      'children',
    ];

    for (const pattern of genrePatterns) {
      const match = query.match(pattern);
      if (match) {
        const potentialGenre = match[1].trim().toLowerCase();
        if (
          commonGenres.some(
            (genre) => potentialGenre.includes(genre) || genre.includes(potentialGenre)
          )
        ) {
          return {
            type: 'genre',
            value: match[1].trim(),
            originalQuery: query,
          };
        }
      }
    }

    // Similar to patterns
    const similarPatterns = [
      /(?:books?\s+)?(?:similar\s+to|like)\s+(.+)/i,
      /(?:more\s+books?\s+like)\s+(.+)/i,
    ];

    for (const pattern of similarPatterns) {
      const match = query.match(pattern);
      if (match) {
        return {
          type: 'similar_to',
          value: match[1].trim(),
          originalQuery: query,
        };
      }
    }

    // Default to general topic search
    return {
      type: 'general',
      value: query,
      originalQuery: query,
    };
  }

  /**
   * Determine search strategy based on query intent
   */
  private getSearchStrategy(intent: QueryIntent): SearchStrategy {
    switch (intent.type) {
      case 'author':
        return {
          useMetadataFilter: true,
          metadataField: 'author',
          metadataValue: intent.value,
          semanticWeight: 0.3, // Lower semantic weight for author searches
          hybridSearch: true,
        };

      case 'genre':
        return {
          useMetadataFilter: true,
          metadataField: 'categories',
          metadataValue: intent.value,
          semanticWeight: 0.7,
          hybridSearch: true,
        };

      case 'similar_to':
      case 'general':
      default:
        return {
          useMetadataFilter: false,
          semanticWeight: 1.0,
          hybridSearch: false,
        };
    }
  }

  /**
   * Perform hybrid search combining metadata filtering and semantic search
   */
  private async performHybridSearch(
    intent: QueryIntent,
    strategy: SearchStrategy,
    topK: number
  ): Promise<any[]> {
    const results: any[] = [];

    if (strategy.useMetadataFilter && strategy.metadataField && strategy.metadataValue) {
      // First, try exact metadata matching
      try {
        const metadataResults = await this.pineconeIndex.query({
          vector: Array(512).fill(0), // Dummy vector for metadata-only search
          topK: topK * 2, // Get more results to filter
          includeMetadata: true,
          filter: {
            [strategy.metadataField]: { $eq: strategy.metadataValue },
          },
        });

        if (metadataResults.matches && metadataResults.matches.length > 0) {
          results.push(...metadataResults.matches);
        }
      } catch (error) {
        console.log('Exact metadata search failed, trying partial match...');
      }

      // If no exact matches or we need more results, try partial matching
      if (results.length < topK) {
        try {
          const partialResults = await this.pineconeIndex.query({
            vector: Array(512).fill(0),
            topK: topK * 2,
            includeMetadata: true,
            filter: {
              [strategy.metadataField]: { $regex: `.*${strategy.metadataValue}.*`, $options: 'i' },
            },
          });

          if (partialResults.matches) {
            // Add results not already included
            const existingIds = new Set(results.map((r) => r.id));
            const newResults = partialResults.matches.filter(
              (match: any) => !existingIds.has(match.id)
            );
            results.push(...newResults);
          }
        } catch (error) {
          console.log('Partial metadata search failed, falling back to semantic search...');
        }
      }
    }

    // If we still don't have enough results or strategy calls for hybrid, do semantic search
    if (results.length < topK || strategy.hybridSearch) {
      const queryEmbedding = await this.model.embed([intent.originalQuery]);
      const embedding = Array.from(queryEmbedding.dataSync()) as number[];

      if (embedding.length !== 512) {
        throw new Error(`Dimension mismatch: expected 512, got ${embedding.length}`);
      }

      const semanticResults = await this.pineconeIndex.query({
        vector: embedding,
        topK: topK * 2,
        includeMetadata: true,
      });

      if (semanticResults.matches) {
        const existingIds = new Set(results.map((r) => r.id));
        const newSemanticResults = semanticResults.matches.filter(
          (match: any) => !existingIds.has(match.id)
        );

        // Weight semantic results if this is a hybrid search
        if (strategy.hybridSearch) {
          newSemanticResults.forEach((result: any) => {
            result.score = (result.score || 0) * strategy.semanticWeight;
          });
        }

        results.push(...newSemanticResults);
      }
    }

    return results;
  }

  /**
   * Post-process and rank results based on intent
   */
  private rankResults(results: any[], intent: QueryIntent, topK: number): any[] {
    let rankedResults = [...results];

    // Apply intent-specific ranking
    switch (intent.type) {
      case 'author':
        rankedResults = rankedResults.sort((a, b) => {
          const aAuthor = (a.metadata?.author || '').toLowerCase();
          const bAuthor = (b.metadata?.author || '').toLowerCase();
          const targetAuthor = intent.value.toLowerCase();

          const aExactMatch = aAuthor.includes(targetAuthor) ? 1 : 0;
          const bExactMatch = bAuthor.includes(targetAuthor) ? 1 : 0;

          if (aExactMatch !== bExactMatch) {
            return bExactMatch - aExactMatch; // Exact matches first
          }

          // Then by original score
          return (b.score || 0) - (a.score || 0);
        });
        break;

      case 'genre':
        rankedResults = rankedResults.sort((a, b) => {
          const aCategories = (a.metadata?.categories || '').toLowerCase();
          const bCategories = (b.metadata?.categories || '').toLowerCase();
          const targetGenre = intent.value.toLowerCase();

          const aMatch = aCategories.includes(targetGenre) ? 1 : 0;
          const bMatch = bCategories.includes(targetGenre) ? 1 : 0;

          if (aMatch !== bMatch) {
            return bMatch - aMatch;
          }

          return (b.score || 0) - (a.score || 0);
        });
        break;

      default:
        rankedResults = rankedResults.sort((a, b) => (b.score || 0) - (a.score || 0));
    }

    // Remove duplicates and limit results
    const seen = new Set();
    return rankedResults
      .filter((result) => {
        const key = `${result.metadata?.title}-${result.metadata?.author}`;
        if (seen.has(key)) return false;
        seen.add(key);
        return true;
      })
      .slice(0, topK);
  }

  async getRecommendations(query: string, topK: number = 10): Promise<Book[]> {
    console.log(`Processing query: "${query}"`);

    // Parse the query to understand user intent
    const intent = this.parseQueryIntent(query);
    console.log(`Detected intent:`, intent);

    // Get search strategy based on intent
    const strategy = this.getSearchStrategy(intent);
    console.log(`Using strategy:`, strategy);

    // Perform hybrid search
    const rawResults = await this.performHybridSearch(intent, strategy, topK);

    // Rank and filter results
    const rankedResults = this.rankResults(rawResults, intent, topK);

    console.log(`Returning ${rankedResults.length} results`);

    return rankedResults.map((match: any) => ({
      title: match.metadata?.title || '',
      author: match.metadata?.author || '',
      description: match.metadata?.description || '',
      categories: match.metadata?.categories || '',
      rating: match.metadata?.rating || '',
      ratingsCount: match.metadata?.ratingsCount || '',
      thumbnail: match.metadata?.thumbnail || '',
      publishedYear: match.metadata?.publishedYear || '',
    }));
  }
}
