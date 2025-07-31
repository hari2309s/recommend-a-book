import express from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import * as tf from '@tensorflow/tfjs-node';
import * as use from '@tensorflow-models/universal-sentence-encoder';
import { Pinecone } from '@pinecone-database/pinecone';
import { RecommendationService } from '@/services/recommendationService';
import { SearchHistoryService } from '@/services/searchHistoryService';
import { RecommendationController } from '@/controllers/recommendationController';
import { setupRoutes } from '@/routes';

dotenv.config();

const app = express();
app.use(cors());
app.use(express.json());

let model: use.UniversalSentenceEncoder;

(async () => {
  try {
    await tf.ready();
    console.log('TensorFlow.js backend initialized');
    model = await use.load();
    console.log('Universal Sentence Encoder model loaded successfully');

    const pinecone = new Pinecone({
      apiKey: process.env.PINECONE_API_KEY!,
    });
    const pineconeIndex = pinecone.Index(process.env.PINECONE_INDEX_NAME!);

    const recommendationService = new RecommendationService(model, pineconeIndex);
    const searchHistoryService = new SearchHistoryService();
    const recommendationController = new RecommendationController(
      recommendationService,
      searchHistoryService
    );

    app.use('/api', setupRoutes(recommendationController));

    const PORT = process.env.PORT || 3000;
    app.listen(PORT, () => console.log(`Server running on port ${PORT}`));
  } catch (error) {
    console.error('Error initializing application:', error);
    process.exit(1);
  }
})();
