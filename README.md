# Book Recommendation System

A full-stack application that provides personalized book recommendations based on user preferences. The system uses advanced natural language processing and vector similarity search to suggest books that match your reading interests.

## Features

- **Semantic Search**: Advanced natural language search powered by sentence embeddings using BAAI/bge-large-en-v1.5 via Hugging Face Inference API
- **Smart Book Recommendations**: Get book suggestions based on semantic understanding of your preferences using vector similarity search
- **Search History**: Keep track of your previous searches and recommendations
- **Fast and Efficient**: Built with Rust for high-performance backend operations
- **Modern UI**: Clean and responsive interface built with React and Radix UI
- **Real-time Updates**: Smooth user experience with infinite scrolling and dynamic loading

## Tech Stack

### Backend (Rust)
- **Web Framework**: Actix-web 4.4
- **Machine Learning**:
  - BAAI/bge-large-en-v1.5 embeddings via Hugging Face Inference API
  - Vector similarity search using Pinecone
- **Database**: Supabase with PostgreSQL
- **Deployment**: Render
- **API Features**:
  - CORS support
  - Structured error handling
  - Configuration management
  - Comprehensive logging and tracing

### Frontend (React)
- **Core**:
  - React 19
  - TypeScript 5.8
  - Vite 6
- **UI/UX**:
  - Radix UI Themes & Components
  - Tailwind CSS
  - Framer Motion for animations
  - Lucide icons
- **Deployment**: Vercel
- **Features**:
  - Fingerprint-based user tracking
  - Responsive design
  - Progressive loading
  - Modern component architecture

## Architecture

The application follows a modern client-server architecture:

- **Backend**: Rust-based API server providing high-performance endpoints for search and recommendations
- **Frontend**: React single-page application with modern state management and UI components
- **ML Pipeline**: Text embedding generation using BAAI/bge-large-en-v1.5 via Hugging Face Inference API and similarity search using Pinecone
- **Data Storage**: Supabase (PostgreSQL) for structured data and search history, Pinecone for vector embeddings
- **Deployment**: Render for hosting the api and Vercel for hosting the frontend

## Environment Variables

### Backend
- `DATABASE_URL`: PostgreSQL connection string
- `PINECONE_API_KEY`: API key for Pinecone vector database
- `HUGGINGFACE_API_KEY`: API key for Hugging Face Inference API
- `RUST_LOG`: Logging level configuration
- Additional configuration in `config/*.toml` files

### Frontend
- `VITE_API_URL`: Backend API endpoint
- `VITE_ENVIRONMENT`: Development/production environment setting
