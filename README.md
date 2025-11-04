# Recommend a Book 📚

> **Discover your next great read with AI-powered RAG semantic search and intelligent recommendations.**

[![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/)
[![Actix Web](https://img.shields.io/badge/Actix%20Web-4.4-blue.svg)](https://actix.rs/)
[![Tokio](https://img.shields.io/badge/Tokio-Async%20Runtime-green.svg)](https://tokio.rs/)
[![React](https://img.shields.io/badge/React-19+-blue.svg)](https://reactjs.org/)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.8+-blue.svg)](https://www.typescriptlang.org/)
[![Vite](https://img.shields.io/badge/Vite-6-purple.svg)](https://vitejs.dev/)
[![Tailwind CSS](https://img.shields.io/badge/Tailwind%20CSS-4-38bdf8.svg)](https://tailwindcss.com/)
[![Radix UI](https://img.shields.io/badge/Radix%20UI-Components-black.svg)](https://www.radix-ui.com/)
[![Pinecone](https://img.shields.io/badge/Pinecone-Vector%20DB-00D4AA.svg)](https://www.pinecone.io/)
[![Hugging Face](https://img.shields.io/badge/🤗%20Hugging%20Face-BGE--large-yellow.svg)](https://huggingface.co/)
[![Performance](https://img.shields.io/badge/Search%20Speed-1--2s-brightgreen)](#performance)
[![License](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)

**Recommend a Book** is a full-stack RAG (Retrieval-Augmented Generation) application that provides personalized book recommendations using advanced natural language processing and vector similarity search. Simply describe what you're looking for, and let AI find books that truly match your reading interests.

## Features

- **Semantic Search**: Advanced natural language search powered by sentence embeddings using BAAI/bge-large-en-v1.5 via Hugging Face Inference API
- **Smart Book Recommendations**: Get book suggestions based on semantic understanding of your preferences using vector similarity search
- **Relevance Indicators**: Each recommendation shows why it was suggested with contextual tags like "Fantasy", "Magic", "Author: Tolkien"
- **Confidence Scores**: See how well each book matches your query with percentage match scores (e.g., "95% match")
- **Semantic Tags**: View extracted themes and topics from your search query to understand what the system detected
- **OpenAPI Documentation**: Interactive API documentation with Swagger UI for easy testing and integration
- **Fast and Efficient**: Built with Rust for high-performance backend operations
- **Modern UI**: Clean and responsive interface built with React 19, Radix UI, and Tailwind CSS
- **Infinite Scrolling**: Smooth user experience with progressive loading and dynamic content
- **Fast Development**: Optimized development scripts with instant startup and hot reloading

## Tech Stack

### Backend (Rust)
- **Web Framework**: Actix-web 4.4 with CORS support
- **Async Runtime**: Tokio for high-performance asynchronous operations
- **Machine Learning**:
  - BAAI/bge-large-en-v1.5 embeddings via Hugging Face Inference API
  - Vector similarity search using Pinecone
- **Vector Database**: Pinecone for embeddings storage and similarity search
- **Deployment**: Render with production configuration
- **API Features**:
  - RESTful API endpoints for recommendations
  - OpenAPI/Swagger documentation for interactive testing
  - Structured error handling with custom error types
  - Configuration management with TOML files
  - Comprehensive logging and tracing
  - Enhanced response data with relevance indicators and confidence scores
  - Semantic tag extraction from user queries

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
  - Interactive book cards with relevance indicators
  - Confidence score visualization
  - Semantic tag display for query understanding
  - Smooth animations and transitions

## Getting Started

### Prerequisites
- Node.js >= 18.0.0
- pnpm 10.14.0
- Rust (latest stable)
- Pinecone & Hugging Face API keys

### Quick Start
```bash
# Clone and install
git clone https://github.com/hari2309s/recommend-a-book.git
cd recommend-a-book
pnpm install
cd apps/api && cargo fetch

# Set up environment
cp apps/api/config/development.toml.example apps/api/config/development.toml
# Edit development.toml with your API keys

# Start development servers
pnpm dev
```

- Frontend: http://localhost:3000
- Backend: http://localhost:10000
- API Docs: http://localhost:10000/swagger-ui/

## API

**Base URL**: `http://localhost:10000` (dev) / `https://recommend-a-book-api.onrender.com` (prod)

### Main Endpoints
- `POST /api/recommendations/` - Get book recommendations
- `GET /api/health` - Health check
- `/swagger-ui/` - Interactive API documentation

### Example Request
```json
{
  "query": "fantasy books with dragons",
  "top_k": 50
}
```

## Scripts

- `pnpm dev` - Start both frontend and backend
- `pnpm build` - Build both applications
- `pnpm index:books` - Index books from CSV

## Deployment

- **API**: [https://recommend-a-book-api.onrender.com](https://recommend-a-book-api.onrender.com) (Render)
- **Frontend**: [https://recommend-a-book-frontend.vercel.app/](https://recommend-a-book-frontend.vercel.app/) (Vercel)

---

**Built with ❤️ from Berlin for book lovers who believe in the power of the perfect recommendation.**
