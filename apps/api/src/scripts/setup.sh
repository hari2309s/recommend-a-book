#!/bin/bash

set -e

echo "Setting up ONNX sentence transformer model..."

# Create models directory
mkdir -p models

# Download all-MiniLM-L6-v2 (384 dimensions, fast and good quality)
echo "Downloading all-MiniLM-L6-v2 model..."

# Model file
curl -L "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/onnx/model.onnx" -o models/model.onnx

# Tokenizer
curl -L "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/tokenizer.json" -o models/tokenizer.json

# Config (optional, for reference)
curl -L "https://huggingface.co/sentence-transformers/all-MiniLM-L6-v2/resolve/main/config.json" -o models/config.json

echo "Model setup complete!"
echo "Model files:"
echo "  - models/model.onnx"
echo "  - models/tokenizer.json"
echo "  - models/config.json"
echo ""
echo "This model outputs 384-dimensional embeddings."
echo "Update your dimension validation accordingly in the Rust code."

# Make script executable
chmod +x scripts/setup-model.sh
