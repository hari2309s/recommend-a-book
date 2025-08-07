import fs from 'fs';
import { parse } from 'csv-parse';
import * as tf from '@tensorflow/tfjs';
import * as use from '@tensorflow-models/universal-sentence-encoder';
import { Pinecone } from '@pinecone-database/pinecone';
import dotenv from 'dotenv';
import { PineconeRecord } from '@/types';

dotenv.config();

const pinecone = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY!,
});
const pineconeIndex = pinecone.Index(process.env.PINECONE_INDEX_NAME!);

/**
 * Normalize author names for better matching
 */
function normalizeAuthor(author: string): string {
  return author
    .trim()
    .replace(/\s+/g, ' ')
    .replace(/[^\w\s.-]/g, '') // Remove special characters except dots and hyphens
    .toLowerCase();
}

/**
 * Extract and normalize categories
 */
function normalizeCategories(categories: string): string {
  return categories
    .trim()
    .toLowerCase()
    .replace(/[;&|]/g, ',') // Replace various separators with commas
    .split(',')
    .map((cat) => cat.trim())
    .filter((cat) => cat.length > 0)
    .join(', ');
}

/**
 * Create a rich text representation for better semantic search
 */
function createSearchableText(book: any): string {
  const parts = [];

  if (book.title) parts.push(`Title: ${book.title}`);
  if (book.author) parts.push(`Author: ${book.author}`);
  if (book.categories) parts.push(`Categories: ${book.categories}`);
  if (book.description) parts.push(`Description: ${book.description}`);

  return parts.join('. ');
}

async function retryUpsert(
  vectors: PineconeRecord[],
  maxRetries: number = 3,
  baseDelayMs: number = 1000
): Promise<boolean> {
  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      await pineconeIndex.upsert(vectors);
      return true;
    } catch (error: any) {
      if (attempt === maxRetries) {
        console.error(`Final upsert attempt ${attempt} failed:`, error.message);
        if (error.name === 'PineconeConnectionError') {
          console.log(
            'Check https://status.pinecone.io/ for outages or verify network connection.'
          );
        }
        return false;
      }
      const delay = baseDelayMs * Math.pow(2, attempt - 1);
      console.warn(`Upsert attempt ${attempt} failed. Retrying in ${delay}ms...`);
      await new Promise((resolve) => setTimeout(resolve, delay));
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

        const title = (row.title || row.Title || '').trim();
        const author = (row.authors || row.Authors || row.author || row.Author || '').trim();
        const description = (row.description || row.Description || '').trim();
        const categories = (row.categories || row.Categories || '').trim();
        const isbn13 = (row.isbn13 || '').trim();
        const publishedYear = (row.published_year || '').trim();
        const ratingsCount = (row.ratings_count || '').trim();
        const rating = (row.average_rating || row.rating || '').trim();
        const thumbnail = (row.image_url || row.thumbnail || '').trim();

        if (title && author) {
          // Require at least title and author
          // Normalize and clean data
          const normalizedAuthor = normalizeAuthor(author);
          const normalizedCategories = normalizeCategories(categories);

          books.push({
            isbn13,
            title,
            author,
            normalizedAuthor, // For searching
            description,
            categories: normalizedCategories,
            publishedYear,
            ratingsCount,
            rating,
            thumbnail,
            searchableText: createSearchableText({
              title,
              author,
              categories: normalizedCategories,
              description,
            }),
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

          // Use rich searchable text for embeddings instead of just description
          const searchableTexts = batch.map(
            (book) => book.searchableText || book.description || ''
          );

          try {
            if (searchableTexts.length === 0) continue;

            const embeddings = await model.embed(searchableTexts);

            const vectors = tf.tidy(() => {
              const vectorPromises: PineconeRecord[] = [];

              for (let j = 0; j < batch.length; j++) {
                const book = batch[j];
                if (j >= embeddings.shape[0]) {
                  console.warn(`Skipping book ${book.title} - no embedding available`);
                  continue;
                }

                const embedding = Array.from(
                  tf.gather(embeddings as unknown as tf.Tensor2D, [j]).dataSync()
                );

                if (embedding.length !== 512) {
                  console.warn(
                    `Unexpected embedding length ${embedding.length} for book: ${book.title}`
                  );
                  continue;
                }

                // Create a unique ID if isbn13 is not available
                const bookId =
                  book.isbn13 ||
                  `${book.title}-${book.author}`.replace(/[^\w-]/g, '-').toLowerCase();

                vectorPromises.push({
                  id: bookId,
                  values: embedding as number[],
                  metadata: {
                    title: book.title,
                    author: book.author,
                    normalizedAuthor: book.normalizedAuthor, // For better searching
                    description: book.description,
                    rating: parseFloat(book.rating) || 0,
                    thumbnail: book.thumbnail,
                    categories: book.categories,
                    publishedYear: parseInt(book.publishedYear) || 0,
                    ratingsCount: parseInt(book.ratingsCount) || 0,
                  },
                });
              }

              return vectorPromises;
            });

            const validVectors = (await Promise.all(vectors)).filter((v) => v !== null);
            if (validVectors.length > 0) {
              console.log(
                `Upserting ${validVectors.length} vectors for batch ${i / batchSize + 1}`
              );

              const success = await retryUpsert(validVectors);
              if (success) {
                console.log(
                  `✅ Indexed batch ${i / batchSize + 1} of ${Math.ceil(books.length / batchSize)} (${validVectors.length} books)`
                );
              } else {
                console.error(`❌ Failed to index batch ${i / batchSize + 1} after retries`);
              }
            }

            // Clean up tensors
            embeddings.dispose();
          } catch (error: any) {
            console.error(
              `Error generating embeddings for batch ${i / batchSize + 1}:`,
              error.message
            );
          }
        }

        console.log('📚 Books indexed successfully');

        // Log some statistics
        const authorsSet = new Set(books.map((b) => b.normalizedAuthor));
        const categoriesSet = new Set(books.flatMap((b) => b.categories.split(', ')));

        console.log(`📊 Statistics:`);
        console.log(`   Total books: ${books.length}`);
        console.log(`   Unique authors: ${authorsSet.size}`);
        console.log(`   Unique categories: ${categoriesSet.size}`);
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
