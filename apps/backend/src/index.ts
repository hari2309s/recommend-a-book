import express from 'express';
import cors from 'cors';
import dotenv from 'dotenv';
import { CohereEmbeddings } from '@langchain/cohere';
import { Pinecone } from '@pinecone-database/pinecone';

dotenv.config();

const app = express();
app.use(cors());
app.use(express.json());

// Initialize Cohere embeddings
const embeddings = new CohereEmbeddings({
  apiKey: process.env.COHERE_API_KEY,
  model: 'embed-english-v3.0',
});

// Initialize Pinecone
const pinecone = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY!,
});
const pineconeIndex = pinecone.Index(process.env.PINECONE_INDEX_NAME!);

// API endpoint for recommendations
app.post('/recommend', async (req, res) => {
  const { query } = req.body;
  if (!query) {
    return res.status(400).json({ error: 'Query is required' });
  }

  try {
    // Generate embedding for the query using Cohere
    const queryEmbedding = await embeddings.embedQuery(query);

    // Query Pinecone for similar books
    const results = await pineconeIndex.query({
      vector: queryEmbedding,
      topK: 5,
      includeMetadata: true,
    });

    // Extract book metadata
    const recommendations = results.matches.map((match) => ({
      title: match.metadata?.title,
      author: match.metadata?.author,
      description: match.metadata?.description,
      categories: match.metadata?.categories,
      rating: match.metadata?.rating,
      ratingsCount: match.metadata?.ratingsCount,
      thumbnail: match.metadata?.thumbnail,
      publishedYear: match.metadata?.publishedYear
    }));

    res.json({ recommendations });
  } catch (error) {
    console.error(error);
    res.status(500).json({ error: 'Failed to fetch recommendations' });
  }
});

app.listen(3000, () => console.log('Server running on port 3000'));
