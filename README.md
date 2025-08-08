# Book Recommendation System

A full-stack application that provides personalized book recommendations based on user preferences. The system uses a dataset from Kaggle to suggest books that match your reading interests.

## Features

- **Semantic Search**: Find books using natural language queries that understand meaning, not just keywords
- **Personalized Recommendations**: Get book suggestions based on semantic understanding of your preferences
- **Modern Tech Stack**: Built with React, TypeScript, Vite, and Node.js
- **Responsive Design**: Works on both desktop and mobile devices
- **Context-Aware Results**: Get more relevant results that understand the context of your search

## Dataset

The book dataset is sourced from [Kaggle](https://www.kaggle.com/). The dataset (`books.csv`) contains information about various books including titles, authors, publication details, and user ratings.

## Tech Stack

- **Frontend**:
  - React 19
  - TypeScript 5.8
  - Vite
  - Tailwind CSS
  - Radix UI Components
  - Framer Motion for animations

- **Backend**:
  - Node.js
  - Express 5
  - TypeScript 5.8
  - TensorFlow.js for machine learning
  - Pinecone for efficient vector similarity search
  - Universal Sentence Encoder for generating semantic embeddings
  - Semantic search pipeline for understanding query context

- **Architecture**:
  - [View System Architecture Diagram](docs/system-architecture.md)

## Getting Started

### Prerequisites

- Node.js (v18 or later)
- pnpm or yarn or npm

### Installation

1. Clone the repository:
   ```bash
   git clone https://github.com/yourusername/recommend-a-book.git
   cd recommend-a-book
   ```

2. Install dependencies for both frontend and backend:
   ```bash
   # Install backend dependencies
   cd apps/backend
   npm install

   # Install frontend dependencies
   cd ../frontend
   npm install
   ```

3. Set up environment variables:
   - Create a `.env` file in the `apps/backend` directory
   - Add any required environment variables (check with the development team if needed)

### Running the Application

1. Start the backend server:
   ```bash
   cd apps/backend
   npm run dev
   ```

2. In a new terminal, start the frontend development server:
   ```bash
   cd apps/frontend
   npm run dev
   ```

3. Open your browser and navigate to `http://localhost:5173`

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- Book dataset provided by [Kaggle](https://www.kaggle.com/)
- Built with amazing open source technologies
