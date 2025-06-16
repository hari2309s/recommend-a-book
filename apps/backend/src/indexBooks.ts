import fs from 'fs';
import { parse } from 'csv-parse';
import * as tf from '@tensorflow/tfjs';
import * as use from '@tensorflow-models/universal-sentence-encoder';
import { Pinecone } from '@pinecone-database/pinecone';
import dotenv from 'dotenv';
import { PineconeRecord } from './types';

dotenv.config();

const pinecone = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY!,
});
const pineconeIndex = pinecone.Index(process.env.PINECONE_INDEX_NAME!);

async function retryUpsert(vectors: PineconeRecord[], maxRetries: number = 3, baseDelayMs: number = 1000): Promise<boolean> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      await pineconeIndex.upsert(vectors);
      return true;
    } catch (error: any) {
      if (attempt === maxRetries) {
        console.error(`Final upsert attempt ${attempt} failed:`, error.message);
        if (error.name === 'PineconeConnectionError') {
          console.log('Check https://status.pinecone.io/ for outages or verify network connection.');
        }
        return false;
      }
      const delay = baseDelayMs * Math.pow(2, attempt - 1);
      console.warn(`Upsert attempt ${attempt} failed. Retrying in ${delay}ms...`);
      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
  return false;
}

async function indexBooks() {
  let model: use.UniversalSentenceEncoder;
  try {
    model = await use.load();
    console.log('Model loaded successfully');
  } catch (error) {
    console.error('Error loading Universal Sentence Encoder model:', error);
    return;
  }

  const books: any[] = [];

  try {
    const stream = fs.createReadStream('data/books.csv');
    stream
      .pipe(parse({ columns: true, skip_empty_lines: true, trim: true, encoding: 'utf8' }))
      .on('data', (row) => {
        console.log('Row data:', JSON.stringify(row, null, 2));

        const title = row.title || row.Title || '';
        const author = row.authors || row.Authors || row.author || row.Author || '';
        const description = row.description || row.Description || '';
        const categories = row.categories || row.Categories || '';
        const isbn13 = row.isbn13 || '';
        const publishedYear = row.published_year || "";
        const ratingsCount = row.ratings_count || '';

        if (title || author || description) {
          books.push({
            isbn13: isbn13.trim(),
            title: title.trim(),
            author: author.trim(),
            description: description.trim(),
            categories: categories.trim(),
            publishedYear: publishedYear.trim(),
            ratingsCount: ratingsCount.trim(),
            rating: (row.average_rating || row.rating || '').trim(),
            thumbnail: (row.image_url || row.thumbnail || '').trim(),
          });
        }
      })
      .on('end', async () => {
        console.log(`Parsed ${books.length} books`);
        if (books.length === 0) {
          console.warn('No books parsed. Check CSV file.');
          return;
        }

        const batchSize = 50;
        for (let i = 0; i < books.length; i += batchSize) {
          const currentBatchSize = Math.min(batchSize, books.length - i);
          const batch = books.slice(i, i + currentBatchSize);
          const descriptions = batch.map((book) => book.description || '');

          try {
            if (descriptions.length === 0) continue;

            const embeddings = await model.embed(descriptions);

            const vectors = tf.tidy(() => {
              const vectorPromises: PineconeRecord[] = [];

              for (let j = 0; j < batch.length; j++) {
                const book = batch[j];
                if (j >= embeddings.shape[0]) {
                  console.warn(`Skipping book ${book.title} - no embedding available`);
                  continue;
                }

                const embedding = Array.from(embeddings.gather([j]).dataSync());

                if (embedding.length !== 512) {
                  console.warn(`Unexpected embedding length ${embedding.length} for book: ${book.title}`);
                }

                vectorPromises.push({
                  id: book.isbn13,
                  values: embedding,
                  metadata: {
                    title: book.title,
                    author: book.author,
                    description: book.description,
                    rating: book.rating,
                    thumbnail: book.thumbnail,
                    categories: book.categories,
                    publishedYear: book.publishedYear,
                    ratingsCount: book.ratingsCount,
                  },
                });
              }

              return vectorPromises;
            });

            const validVectors = (await Promise.all(vectors)).filter((v) => v !== null);
            if (validVectors.length > 0) {
              const success = await retryUpsert(validVectors);
              if (success) {
                console.log(`Indexed batch ${i / batchSize + 1} of ${Math.ceil(books.length / batchSize)}`);
              } else {
                console.error(`Failed to index batch ${i / batchSize + 1} after retries`);
              }
            }
          } catch (error: any) {
            console.error(`Error generating embeddings for batch ${i / batchSize + 1}:`, error.message);
          }
        }

        console.log('Books indexed successfully');
      })
      .on('error', (error) => {
        console.error('Error parsing CSV:', error);
      });
  } catch (error) {
    console.error('File access error:', error);
  } finally {
    if (model) {
      tf.dispose();
    }
  }
}

indexBooks().catch(console.error);
