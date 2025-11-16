#!/bin/bash

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

print_info "Starting graph building process..."

# Check for required environment variables
REQUIRED_VARS=(
    "APP_HUGGINGFACE_API_KEY"
    "APP_PINECONE_API_KEY"
    "APP_PINECONE_ENV"
    "APP_PINECONE_INDEX_NAME"
    "APP_NEO4J_PASSWORD"
)

print_info "Checking environment variables..."
for var in "${REQUIRED_VARS[@]}"; do
    if [ -z "${!var}" ]; then
        print_error "Missing required environment variable: $var"
        echo ""
        echo "Required environment variables:"
        echo "  - APP_HUGGINGFACE_API_KEY: Your HuggingFace API key"
        echo "  - APP_PINECONE_API_KEY: Your Pinecone API key"
        echo "  - APP_PINECONE_ENV: Your Pinecone environment"
        echo "  - APP_PINECONE_INDEX_NAME: Your Pinecone index name"
        echo "  - APP_NEO4J_URI: Neo4j connection URI (default: bolt://localhost:7687)"
        echo "  - APP_NEO4J_USER: Neo4j username (default: neo4j)"
        echo "  - APP_NEO4J_PASSWORD: Neo4j password"
        echo ""
        echo "You can set these in your .env file or export them:"
        echo "export $var=your_value_here"
        exit 1
    else
        # Show partial value for security
        value="${!var}"
        masked_value="${value:0:5}...${value: -3}"
        print_info "✓ $var: $masked_value"
    fi
done

# Check if we're in the right directory (should contain Cargo.toml)
if [ ! -f "Cargo.toml" ]; then
    print_error "Cargo.toml not found. Please run this script from the API project root directory."
    exit 1
fi

# Load environment variables from .env if it exists
if [ -f ".env" ]; then
    print_info "Loading environment variables from .env file..."
    export $(cat .env | xargs)
fi

# Set default Neo4j values if not set
export APP_NEO4J_URI=${APP_NEO4J_URI:-"bolt://localhost:7687"}
export APP_NEO4J_USER=${APP_NEO4J_USER:-"neo4j"}

print_info "Neo4j Configuration:"
print_info "  URI: $APP_NEO4J_URI"
print_info "  User: $APP_NEO4J_USER"

# Ask if user wants to clear existing graph
echo ""
read -p "Do you want to clear the existing graph before building? (y/N): " clear_graph
if [[ $clear_graph =~ ^[Yy]$ ]]; then
    export CLEAR_GRAPH=true
    print_warning "Existing graph will be cleared"
else
    export CLEAR_GRAPH=false
    print_info "Existing graph will be preserved, new relationships will be added"
fi

# Build the project
print_info "Building the graph building binary..."
cargo build --bin build_graph --release

if [ $? -ne 0 ]; then
    print_error "Failed to build the project"
    exit 1
fi

print_success "Build completed successfully!"

print_warning "This process will:"
print_warning "  1. Fetch all books from Pinecone"
print_warning "  2. Add books as nodes to Neo4j"
print_warning "  3. Create semantic relationships between books"
print_warning "  4. This may take 30-60 minutes depending on your dataset size"

echo ""
read -p "Do you want to continue with graph building? (y/N): " confirm
if [[ ! $confirm =~ ^[Yy]$ ]]; then
    print_info "Graph building cancelled by user"
    exit 0
fi

print_info "Starting graph building process..."
echo "=================================================================================="

# Set logging level
export RUST_LOG="build_graph=info,recommend_a_book_api=info"

# Run the graph builder
./target/release/build_graph

# Check exit status
if [ $? -eq 0 ]; then
    echo "=================================================================================="
    print_success "Graph building completed successfully!"
    print_info "Your book graph is now ready for queries"
    print_info "You can visualize it using Neo4j Browser at http://localhost:7474"
else
    echo "=================================================================================="
    print_error "Graph building failed. Check the logs above for details."
    exit 1
fi
