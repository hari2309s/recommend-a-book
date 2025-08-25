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

# Check if CSV file argument is provided
if [ $# -eq 0 ]; then
    print_error "No CSV file provided!"
    echo "Usage: $0 <path_to_csv_file>"
    echo "Example: $0 ./data/books.csv"
    exit 1
fi

CSV_FILE="$1"

# Check if CSV file exists
if [ ! -f "$CSV_FILE" ]; then
    print_error "CSV file does not exist: $CSV_FILE"
    exit 1
fi

print_info "Starting book indexing process..."
print_info "CSV file: $CSV_FILE"

# Check for required environment variables
REQUIRED_VARS=(
    "APP_HUGGINGFACE_API_KEY"
    "APP_PINECONE_API_KEY"
    "APP_PINECONE_ENV"
    "APP_PINECONE_INDEX_NAME"
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

# Build the project
print_info "Building the indexing binary..."
cargo build --bin index_books --release

if [ $? -ne 0 ]; then
    print_error "Failed to build the project"
    exit 1
fi

print_success "Build completed successfully!"

# Get file info
file_size=$(wc -l < "$CSV_FILE")
print_info "CSV file contains approximately $file_size lines"

# Estimate processing time
estimated_minutes=$((file_size / 1000))
if [ $estimated_minutes -eq 0 ]; then
    estimated_minutes=1
fi

print_warning "Estimated processing time: ~$estimated_minutes minutes"
print_warning "This will use your HuggingFace and Pinecone API quotas"

# Ask for confirmation
echo ""
read -p "Do you want to continue with indexing? (y/N): " confirm
if [[ ! $confirm =~ ^[Yy]$ ]]; then
    print_info "Indexing cancelled by user"
    exit 0
fi

print_info "Starting indexing process..."
echo "=================================================================================="

# Set logging level
export RUST_LOG="index_books=info,recommend_a_book_api=info"

# Run the indexing
./target/release/index_books "$CSV_FILE"

# Check exit status
if [ $? -eq 0 ]; then
    echo "=================================================================================="
    print_success "Indexing completed successfully!"
    print_info "Your books are now indexed and ready for recommendations"
else
    echo "=================================================================================="
    print_error "Indexing failed. Check the logs above for details."
    exit 1
fi
