#!/bin/bash
set -e

# Colors for output
YELLOW='\033[1;33m'
GREEN='\033[1;32m'
RED='\033[1;31m'
BLUE='\033[1;34m'
NC='\033[0m' # No Color

echo -e "${BLUE}====================================================${NC}"
echo -e "${BLUE}  Recommend-a-Book API Development Setup Script     ${NC}"
echo -e "${BLUE}====================================================${NC}"

# Check if running from the correct directory
if [ ! -d "config" ]; then
  echo -e "${RED}Error: This script must be run from the api directory${NC}"
  echo -e "${YELLOW}Please run this script from the apps/api directory${NC}"
  exit 1
fi

echo -e "\n${YELLOW}Checking prerequisites...${NC}"

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
  echo -e "${RED}Error: Rust/Cargo is not installed${NC}"
  echo -e "${YELLOW}Please install Rust from https://rustup.rs/${NC}"
  exit 1
fi

echo -e "${GREEN}✓ Rust is installed${NC}"

# Create configuration files from examples
echo -e "\n${YELLOW}Setting up configuration files...${NC}"

if [ ! -f "config/development.toml" ]; then
  if [ -f "config/development.toml.example" ]; then
    cp config/development.toml.example config/development.toml
    echo -e "${GREEN}✓ Created config/development.toml${NC}"
  else
    echo -e "${RED}Error: config/development.toml.example not found${NC}"
    exit 1
  fi
else
  echo -e "${GREEN}✓ config/development.toml already exists${NC}"
fi

if [ ! -f "config/local.toml" ]; then
  if [ -f "config/local.toml.example" ]; then
    cp config/local.toml.example config/local.toml
    echo -e "${GREEN}✓ Created config/local.toml${NC}"
  else
    echo -e "${RED}Warning: config/local.toml.example not found, creating empty file${NC}"
    touch config/local.toml
  fi
else
  echo -e "${GREEN}✓ config/local.toml already exists${NC}"
fi

# Build the project
echo -e "\n${YELLOW}Building the project...${NC}"
cargo build

# Success message and next steps
echo -e "\n${GREEN}==============================================${NC}"
echo -e "${GREEN}  Development environment setup complete!     ${NC}"
echo -e "${GREEN}==============================================${NC}"

echo -e "\n${YELLOW}Next steps:${NC}"
echo -e "1. Edit ${BLUE}config/local.toml${NC} to add your API keys:"
echo -e "   - Supabase URL and key"
echo -e "   - Pinecone API key, environment, and index"
echo -e "   - HuggingFace API key (if needed)"
echo -e "\n2. Or set environment variables (recommended for security):"
echo -e "   export APP_SUPABASE_URL=\"your-url\""
echo -e "   export APP_SUPABASE_KEY=\"your-key\""
echo -e "   export APP_PINECONE_API_KEY=\"your-key\""
echo -e "   export APP_PINECONE_ENV=\"your-env\""
echo -e "   export APP_PINECONE_INDEX_NAME=\"your-index\""
echo -e "\n3. Run the API locally:"
echo -e "   cargo run"
echo -e "\n${BLUE}For more details, see DEPLOYMENT.md${NC}"
