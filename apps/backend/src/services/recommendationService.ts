import * as use from '@tensorflow-models/universal-sentence-encoder';
import { Book } from '@/types';

export class RecommendationService {
  private model: use.UniversalSentenceEncoder;
  private pineconeIndex: any;

  constructor(model: use.UniversalSentenceEncoder, pineconeIndex: any) {
    this.model = model;
    this.pineconeIndex = pineconeIndex;
  }

  async getRecommendations(query: string): Promise<Book[]> {
    const queryEmbedding = await this.model.embed([query]);
    const embeddingTensor = queryEmbedding.slice([0, 0], [1, 512]);
    const embedding = Array.from(embeddingTensor.dataSync()) as number[];

    if (embedding.length !== 512) {
      throw new Error(`Dimension mismatch: expected 512, got ${embedding.length}`);
    }

    const results = await this.pineconeIndex.query({
      vector: embedding,
      topK: 10,
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
