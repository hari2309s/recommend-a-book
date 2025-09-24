# Semantic Book Recommendation System

A full-stack application that provides personalized book recommendations based on user preferences. The system uses advanced natural language processing and vector similarity search to suggest books that match your reading interests.

## Features

- **Semantic Search**: Advanced natural language search powered by sentence embeddings using BAAI/bge-large-en-v1.5 via Hugging Face Inference API
- **Smart Book Recommendations**: Get book suggestions based on semantic understanding of your preferences using vector similarity search
- **OpenAPI Documentation**: Interactive API documentation with Swagger UI for easy testing and integration
- **Fast and Efficient**: Built with Rust for high-performance backend operations
- **Modern UI**: Clean and responsive interface built with React 19, Radix UI, and Tailwind CSS
- **Infinite Scrolling**: Smooth user experience with progressive loading and dynamic content
- **Fast Development**: Optimized development scripts with instant startup and hot reloading

## Tech Stack

### Backend (Rust)
- **Web Framework**: Actix-web 4.4 with CORS support
- **Machine Learning**:
  - BAAI/bge-large-en-v1.5 embeddings via Hugging Face Inference API
  - Vector similarity search using Pinecone
- **Database**: Supabase with PostgreSQL for structured data
- **Vector Database**: Pinecone for embeddings storage and similarity search
- **Deployment**: Render with production configuration
- **API Features**:
  - RESTful API endpoints for recommendations
  - OpenAPI/Swagger documentation for interactive testing
  - Structured error handling with custom error types
  - Configuration management with TOML files
  - Comprehensive logging and tracing

### Frontend (React)
- **Core**:
  - React 19 with TypeScript 5.8
  - Vite 6 for build tooling and development server
- **UI/UX**:
  - Radix UI Themes & Components for accessible design
  - Tailwind CSS 4.x for styling
  - Framer Motion for smooth animations
  - Lucide React for consistent iconography
- **State Management**: Custom hooks with infinite scroll
- **Deployment**: Vercel with optimized build configuration
- **Features**:
  - Responsive design with mobile-first approach
  - Infinite scrolling with progressive loading
  - Modern component architecture with TypeScript
  - Error handling and loading states

## Architecture

The application follows a modern client-server architecture with a monorepo structure:

- **Backend**: Rust-based API server providing high-performance endpoints for search and recommendations
- **Frontend**: React single-page application with modern state management and UI components
- **ML Pipeline**: Text embedding generation using BAAI/bge-large-en-v1.5 via Hugging Face Inference API and similarity search using Pinecone
- **Data Storage**:
  - Supabase (PostgreSQL) for structured data
  - Pinecone for vector embeddings and similarity search
- **Deployment**:
  - Render for hosting the Rust API
  - Vercel for hosting the React frontend
- **Monorepo**: Managed with pnpm workspaces and Turbo for build orchestration

## Getting Started

### Prerequisites

- **Node.js**: >= 18.0.0
- **pnpm**: 10.14.0 (package manager)
- **Rust**: Latest stable version
- **PostgreSQL**: For local development
- **API Keys**: Pinecone, Hugging Face, and Supabase accounts

### Installation

1. **Clone the repository**
   ```bash
   git clone https://github.com/hari2309s/recommend-a-book.git
   cd recommend-a-book
   ```

2. **Install dependencies**
   ```bash
   pnpm install
   cd apps/api && cargo fetch
   ```

3. **Set up environment variables**
   ```bash
   # Copy example configuration files
   cp apps/api/config/development.toml.example apps/api/config/development.toml

   # Set up environment variables (see Environment Variables section)
   ```

4. **Start development servers**
   ```bash
   # Start both frontend and backend in development mode
   pnpm dev

   # Or start individually
   pnpm dev:frontend  # Frontend on http://localhost:3000
   pnpm dev:api       # Backend on http://localhost:10000

   # For production-like testing (API in dev mode, frontend built)
   pnpm start
   ```

   After starting the backend, you can access the API documentation at:
   - Swagger UI: `http://localhost:10000/swagger-ui/`
   - OpenAPI JSON: `http://localhost:10000/api-doc/openapi.json`

### Book Indexing

To index books from a CSV file:

```bash
# Using the convenient script
pnpm index:books

# Or manually
cd apps/api
cargo run --bin index_books -- data/books.csv
```

## API Endpoints

**Base URL**: `http://localhost:10000` (development) / `https://recommend-a-book-api.onrender.com` (production)

### Recommendations
- `POST /api/recommendations/` - Get book recommendations based on query

### Health Check
- `GET /api/health` - API health status

### API Documentation
- `/swagger-ui/` - Interactive Swagger UI documentation
- `/api-doc/openapi.json` - OpenAPI specification in JSON format

#### Using Swagger UI
Once the server is running, you can access the Swagger UI documentation:

1. Navigate to `http://localhost:10000/swagger-ui/` in your browser (using your configured port)
2. The UI shows all available endpoints with their descriptions, parameters, and response formats
3. To test an endpoint:
   - Click on the endpoint to expand it
   - Click the "Try it out" button
   - Fill in any required parameters or request body
   - Click "Execute" to make a real API call
   - View the response directly in the UI

This makes it easy to explore and test the API without writing any code.

## Environment Variables

### Backend
- `APP_DATABASE_URL`: PostgreSQL connection string
- `APP_PINECONE_API_KEY`: API key for Pinecone vector database
- `APP_PINECONE_ENV`: Pinecone environment
- `APP_PINECONE_INDEX_NAME`: Pinecone index name
- `APP_HUGGINGFACE_API_KEY`: API key for Hugging Face Inference API
- `APP_SUPABASE_URL`: Supabase project URL
- `APP_SUPABASE_KEY`: Supabase anon key
- `RUST_LOG`: Logging level configuration

### Frontend
- `VITE_API_URL`: Backend API endpoint for development
- `VITE_RECOMMEND_A_BOOK_API_BASE_URL`: Development API base URL (default: http://localhost:10000)
- `VITE_RECOMMEND_A_BOOK_API_PROD_BASE_URL`: Production API base URL (https://recommend-a-book-api.onrender.com)
- `VITE_ENVIRONMENT`: Development/production environment setting

### Available Scripts

- `pnpm dev` - Start both frontend and backend in development mode
- `pnpm dev:frontend` - Start only the frontend development server
- `pnpm dev:api` - Start only the backend development server
- `pnpm build` - Build both applications
- `pnpm build:frontend` - Build only the frontend
- `pnpm build:api` - Build only the backend
- `pnpm start` - Start both applications (API in dev mode, frontend built and previewed)
- `pnpm start:api` - Start only the backend API server
- `pnpm start:frontend` - Build and preview the frontend
- `pnpm lint` - Run linting across the project
- `pnpm lint:frontend` - Run linting only on frontend
- `pnpm format` - Format code with Prettier
- `pnpm clean` - Clean build artifacts and dependencies
- `pnpm index:books` - Index books from CSV file

### Database Setup

1. **Supabase Setup**:
   - Create a new Supabase project
   - Set up the database schema for search history
   - Configure environment variables

2. **Pinecone Setup**:
   - Create a Pinecone account and index
   - Configure the index for vector similarity search
   - Set up environment variables

## Deployment

### Backend (Render)

The backend is deployed on Render with the following setup:

- **Live URL**: [https://recommend-a-book-api.onrender.com](https://recommend-a-book-api.onrender.com)
- **Runtime**: Rust
- **Build Command**: `cargo build --release`
- **Start Command**: `./target/release/recommend-a-book-api`
- **Health Check**: `/api/health`
- **Documentation**: `/swagger-ui/`

### Frontend (Vercel)

The frontend is deployed on Vercel with:

- **Live URL**: [https://recommend-a-book-frontend.vercel.app/](https://recommend-a-book-frontend.vercel.app/)
- **Build Command**: `pnpm run build`
- **Output Directory**: `dist`
- **Environment Variables**: Configured for production API URL
