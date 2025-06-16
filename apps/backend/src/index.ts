import express from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import * as tf from '@tensorflow/tfjs-node';
import * as use from '@tensorflow-models/universal-sentence-encoder';
import { Pinecone } from '@pinecone-database/pinecone';

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
  } catch (error) {
    console.error('Error loading Universal Sentence Encoder model:', error);
    process.exit(1);
  }
})();

const pinecone = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY!,
});
const pineconeIndex = pinecone.Index(process.env.PINECONE_INDEX_NAME!);

app.post('/recommend', async (req, res) => {
  const { query } = req.body;

  if (!query) {
    res.status(400).json({ error: 'Query is required' });
    return;
  }

  try {
    const queryEmbedding = await model.embed([query]);
    const embeddingTensor = queryEmbedding.slice([0, 0], [1, 512]);
    const embedding = Array.from(embeddingTensor.dataSync()) as number[];

    if (embedding.length !== 512) {
      throw new Error(`Dimension mismatch: expected 512, got ${embedding.length}`);
    }

    const results = await pineconeIndex.query({
      vector: embedding,
      topK: 10,
      includeMetadata: true,
    });

    const recommendations = results.matches.map((match) => ({
      title: match.metadata?.title as string,
      author: match.metadata?.author as string,
      description: match.metadata?.description as string,
      categories: match.metadata?.categories as string,
      rating: match.metadata?.rating as string,
      ratingsCount: match.metadata?.ratingsCount as string,
      thumbnail: match.metadata?.thumbnail as string,
      publishedYear: match.metadata?.publishedYear as string,
    }));

    res.json({ recommendations });
  } catch (error) {
    console.error('Error fetching recommendations:', error);
    res.status(500).json({ error: 'Failed to fetch recommendations' });
  }
});

app.listen(3000, () => console.log('Server running on port 3000'));
