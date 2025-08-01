import * as use from '@tensorflow-models/universal-sentence-encoder';
import * as tf from '@tensorflow/tfjs-node';
import { Book } from '@/types';

export class RecommendationService {
  private model: use.UniversalSentenceEncoder;
  private pineconeIndex: any;

  constructor(model: use.UniversalSentenceEncoder, pineconeIndex: any) {
    this.model = model;
    this.pineconeIndex = pineconeIndex;
  }

  async getRecommendations(query: string, topK: number = 10): Promise<Book[]> {
    const queryEmbedding = await this.model.embed([query]);
    const embedding = Array.from(queryEmbedding.dataSync()) as number[];

    if (embedding.length !== 512) {
      throw new Error(`Dimension mismatch: expected 512, got ${embedding.length}`);
    }

    const results = await this.pineconeIndex.query({
      vector: embedding,
      topK,
      includeMetadata: true,
    });

    return results.matches.map((match: any) => ({
      title: match.metadata?.title as string,
      author: match.metadata?.author as string,
      description: match.metadata?.description as string,
      categories: match.metadata?.categories as string,
      rating: match.metadata?.rating as string,
      ratingsCount: match.metadata?.ratingsCount as string,
      thumbnail: match.metadata?.thumbnail as string,
      publishedYear: match.metadata?.publishedYear as string,
    }));
  }
}
