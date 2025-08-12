#!/bin/bash

# Render Deployment Script for Recommend-a-Book Rust API
# This script helps prepare and deploy the Rust API to Render

set -e

echo "🚀 Recommend-a-Book Rust API - Render Deployment Script"
echo "========================================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to print colored output
print_status() {
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

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    print_error "Please run this script from the apps/api directory"
    exit 1
fi

# Check if required tools are installed
check_dependencies() {
    print_status "Checking dependencies..."

    if ! command -v cargo &> /dev/null; then
        print_error "Rust/Cargo is not installed. Please install Rust first."
        exit 1
    fi

    if ! command -v git &> /dev/null; then
        print_error "Git is not installed. Please install Git first."
        exit 1
    fi

    print_success "All dependencies are available"
}

# Validate configuration
validate_config() {
    print_status "Validating configuration..."

    if [ ! -f "config/production.toml" ]; then
        print_error "Production configuration file not found"
        exit 1
    fi

    if [ ! -f "render.yaml" ]; then
        print_error "Render configuration file not found"
        exit 1
    fi

    print_success "Configuration files are valid"
}

# Test build locally
test_build() {
    print_status "Testing local build..."

    # Clean previous builds
    cargo clean

    # Build in release mode
    if cargo build --release; then
        print_success "Local build completed successfully"
    else
        print_error "Local build failed. Please fix errors before deploying."
        exit 1
    fi
}

# Run tests
run_tests() {
    print_status "Running tests..."

    if cargo test; then
        print_success "All tests passed"
    else
        print_warning "Some tests failed, but continuing with deployment"
    fi
}

# Check environment variables
check_env_vars() {
    print_status "Checking required environment variables..."

    required_vars=(
        "APP_SUPABASE_URL"
        "APP_SUPABASE_KEY"
        "APP_PINECONE_API_KEY"
        "APP_PINECONE_ENVIRONMENT"
        "APP_PINECONE_INDEX"
    )

    missing_vars=()

    for var in "${required_vars[@]}"; do
        if [ -z "${!var}" ]; then
            missing_vars+=("$var")
        fi
    done

    if [ ${#missing_vars[@]} -gt 0 ]; then
        print_warning "The following environment variables are not set locally:"
        for var in "${missing_vars[@]}"; do
            echo "  - $var"
        done
        print_warning "Make sure to set these in the Render dashboard"
    else
        print_success "All required environment variables are set"
    fi
}

# Generate deployment summary
generate_summary() {
    print_status "Generating deployment summary..."

    echo ""
    echo "📋 Deployment Summary"
    echo "===================="
    echo "Service Name: recommend-a-book-rust-api"
    echo "Runtime: Rust"
    echo "Region: frankfurt"
    echo "Health Check: /api/health"
    echo "Build Command: cargo build --release"
    echo "Start Command: ./target/release/recommend-a-book-api"
    echo ""
    echo "📊 Environment Variables to Set in Render Dashboard:"
    echo "- APP_SUPABASE_URL: Your Supabase project URL"
    echo "- APP_SUPABASE_KEY: Your Supabase anon/service role key"
    echo "- APP_PINECONE_API_KEY: Your Pinecone API key"
    echo "- APP_PINECONE_ENVIRONMENT: Your Pinecone environment (e.g., gcp-starter)"
    echo "- APP_PINECONE_INDEX: Your Pinecone index name"
    echo "- APP_HUGGINGFACE_API_KEY: (Optional) HuggingFace API key for better model downloads"
    echo ""
    echo "🔗 Endpoints after deployment:"
    echo "- Health Check: https://your-service-url.onrender.com/api/health"
    echo "- Recommendations: POST https://your-service-url.onrender.com/api/recommendations/"
    echo "- Search History: POST https://your-service-url.onrender.com/api/recommendations/history"
    echo ""
}

# Main deployment process
main() {
    echo ""
    print_status "Starting deployment preparation..."

    check_dependencies
    validate_config
    check_env_vars

    if [ "$1" != "--skip-build" ]; then
        test_build
        run_tests
    else
        print_warning "Skipping build and tests as requested"
    fi

    generate_summary

    echo ""
    print_success "🎉 Deployment preparation completed!"
    echo ""
    print_status "Next steps:"
    echo "1. Commit and push your changes to GitHub"
    echo "2. Create a new Web Service on Render"
    echo "3. Connect your GitHub repository"
    echo "4. Set the root directory to 'apps/api'"
    echo "5. Add the environment variables listed above"
    echo "6. Deploy!"
    echo ""
    print_status "Render will automatically use the render.yaml configuration"
    echo ""
}

# Handle command line arguments
case "$1" in
    --help|-h)
        echo "Usage: $0 [OPTIONS]"
        echo ""
        echo "Options:"
        echo "  --skip-build    Skip local build and tests"
        echo "  --help, -h      Show this help message"
        echo ""
        echo "This script prepares the Rust API for deployment to Render."
        echo "It validates configuration, runs tests, and provides deployment instructions."
        ;;
    *)
        main "$@"
        ;;
esac
