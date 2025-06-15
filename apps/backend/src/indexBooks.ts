import fs from 'fs';
import { parse } from 'csv-parse';
import { Pinecone } from '@pinecone-database/pinecone';
import * as tf from '@tensorflow/tfjs';
import * as use from '@tensorflow-models/universal-sentence-encoder';
import dotenv from 'dotenv';
import type { Book, PineconeRecord } from './types';

dotenv.config();

// Initialize Pinecone
const pinecone = new Pinecone({
  apiKey: process.env.PINECONE_API_KEY!,
});
const pineconeIndex = pinecone.Index(process.env.PINECONE_INDEX_NAME!);


async function indexBooks() {
  let model: use.UniversalSentenceEncoder;
  try {
    model = await use.load();
    console.log('Model loaded successfully');
  } catch (error) {
    console.error('Error loading Universal Sentence Encoder model:', error);
    return;
  }

  const books: Book[] = [];

  // Read and parse the CSV file with debugging
  try {
    const stream = fs.createReadStream('data/books.csv');
    stream
      .pipe(
        parse({
          columns: true,
          skip_empty_lines: true,
          trim: true,
          encoding: 'utf8',
        })
      )
      .on('data', (row) => {
        console.log('Row data:', JSON.stringify(row, null, 2)); // Log full row structure
        // Flexible field mapping based on Kaggle dataset
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
          console.log(`Added book: ${title} by ${author}`);
        } else {
          console.log('Skipping row due to no usable fields:', JSON.stringify(row, null, 2));
        }
      })
      .on('end', async () => {
        console.log(`Parsed ${books.length} books`);

        if (books.length === 0) {
          console.warn('No books were parsed. Check CSV file, headers, or data content.');
          return;
        }

        // Index books in batches to avoid rate limits
        const batchSize = 50; // Reduced batch size to be safer
        for (let i = 0; i < books.length; i += batchSize) {
          const currentBatchSize = Math.min(batchSize, books.length - i);
          const batch = books.slice(i, i + currentBatchSize);
          const descriptions = batch.map((book) => book.description || '');
          
          if (descriptions.length === 0) continue;
          
          const embeddings = await model.embed(descriptions);
          // Process embeddings in a single tf.tidy block
          const vectors = tf.tidy(() => {
            const vectorPromises: PineconeRecord[] = [];

            // Process each book's embedding
            for (let j = 0; j < batch.length; j++) {
              const book = batch[j];
              if (j >= embeddings.shape[0]) {
                console.warn(`Skipping book ${book.title} - no embedding available`);
                continue;
              }
              // Get the embedding for this book
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

          // Upsert the vectors to Pinecone
          await pineconeIndex.upsert(vectors);
          //   return {
          //     id: book.isbn13,
          //     values: embedding,
          //     metadata: {
          //       title: book.title,
          //       author: book.author,
          //       description: book.description,
          //       rating: book.rating,
          //       thumbnail: book.thumbnail,
          //       categories: book.categories,
          //       publishedYear: book.publishedYear,
          //       ratingsCount: book.ratingsCount,
          //     },
          //   };
          // })

          await pineconeIndex.upsert(vectors);
          console.log(`Indexed batch ${i / batchSize + 1} of ${Math.ceil(books.length / batchSize)}`);
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
      tf.dispose(); // Clean up TensorFlow resources
    }
  }
}

indexBooks().catch(console.error);
