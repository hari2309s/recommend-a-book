# Render Deployment Guide - Recommend-a-Book Rust API

This guide walks you through deploying the Rust API to Render.com.

## 🚀 Quick Start

1. **Prepare for deployment**:
   ```bash
   cd apps/api
   ./scripts/deploy.sh
   ```

2. **Deploy to Render**:
   - Create a new Web Service on [Render](https://dashboard.render.com)
   - Connect your GitHub repository
   - Set root directory to `apps/api`
   - Configure environment variables (see below)

## 📋 Pre-requisites

- ✅ Rust 1.75+
- ✅ Git repository pushed to GitHub
- ✅ Render account
- ✅ Supabase project with database set up
- ✅ Pinecone index created and configured
- ✅ Required API keys (see Environment Variables section)

## 🔧 Render Configuration

The service is configured via `render.yaml`:

```yaml
services:
  - type: web
    name: recommend-a-book-rust-api
    runtime: rust
    region: frankfurt
    plan: starter
    buildCommand: cargo build --release
    startCommand: ./target/release/recommend-a-book-api
    healthCheckPath: /api/health
```

## 🌍 Environment Variables

Set these in the Render dashboard under your service settings:

### Required Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `APP_SUPABASE_URL` | Your Supabase project URL | `https://abc123.supabase.co` |
| `APP_SUPABASE_KEY` | Supabase anon/service role key | `eyJhbGci...` |
| `APP_PINECONE_API_KEY` | Pinecone API key | `pcsk_123...` |
| `APP_PINECONE_ENVIRONMENT` | Pinecone environment | `gcp-starter` |
| `APP_PINECONE_INDEX` | Pinecone index name | `books-index` |

### Optional Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `APP_HUGGINGFACE_API_KEY` | HuggingFace API for faster model downloads | Not set |
| `APP_FRONTEND_URL` | Frontend URL for CORS | `https://recommend-a-book-frontend.vercel.app` |
| `RUST_LOG` | Logging level | `info` |

### Auto-configured Variables

These are set automatically by the `render.yaml`:

- `RUN_MODE=production`
- `APP_HOST=0.0.0.0`
- `APP_PORT=10000`
- `APP_ENVIRONMENT=production`

## 🏗️ Build Process

Render will automatically:

1. **Install Rust toolchain** (1.75+)
2. **Install system dependencies** (OpenSSL, PostgreSQL client, etc.)
3. **Build the application**: `cargo build --release`
4. **Start the service**: `./target/release/recommend-a-book-api`

Build time: ~5-10 minutes (ML dependencies are large)

## 🔍 Health Checks

The service includes a health check endpoint at `/api/health`:

```json
{
  "status": "healthy",
  "service": "recommend-a-book-api",
  "timestamp": "2024-01-12T10:30:00Z"
}
```

## 📡 API Endpoints

After deployment, your API will be available at:
`https://your-service-name.onrender.com`

### Available Endpoints

- **Health Check**: `GET /api/health`
- **Get Recommendations**: `POST /api/recommendations/`
- **Search History**: `POST /api/recommendations/history`

### Example Usage

```bash
# Test health endpoint
curl https://your-service-name.onrender.com/api/health

# Get book recommendations
curl -X POST https://your-service-name.onrender.com/api/recommendations/ \
  -H "Content-Type: application/json" \
  -d '{
    "query": "science fiction books like Dune",
    "top_k": 5,
    "user_id": "123e4567-e89b-12d3-a456-426614174000"
  }'
```

## 🔐 Security

- All sensitive data is passed via environment variables
- Service runs as non-root user
- CORS is configured for your frontend domain
- API keys are never logged or exposed

## 📊 Performance & Scaling

### Resource Requirements

- **Memory**: ~1-2GB (ML models are memory-intensive)
- **CPU**: 1-2 cores recommended
- **Storage**: ~500MB (models + application)
- **Cold start**: ~30-60 seconds (model loading)

### Scaling Options

- **Render Starter Plan**: 0.5GB RAM, sufficient for development
- **Render Standard Plan**: 2GB+ RAM, recommended for production
- **Auto-scaling**: Configure based on CPU/memory usage

## 🐛 Troubleshooting

### Common Issues

#### 1. Build Timeouts
**Problem**: Cargo build times out
**Solution**:
- Use Render Standard plan or higher
- Enable cargo caching (automatic on Render)

#### 2. Model Loading Failures
**Problem**: ML models fail to download
**Solution**:
- Check internet connectivity
- Verify HuggingFace API key (if using)
- Increase startup timeout

#### 3. Pinecone Connection Errors
**Problem**: "PineconeError: API returned 401"
**Solution**:
- Verify `APP_PINECONE_API_KEY` is correct
- Check Pinecone index name matches `APP_PINECONE_INDEX`
- Ensure Pinecone environment is correct

#### 4. Supabase Connection Issues
**Problem**: Database connection failures
**Solution**:
- Verify `APP_SUPABASE_URL` and `APP_SUPABASE_KEY`
- Check Supabase project is active
- Verify database tables exist

### Viewing Logs

Access logs in Render dashboard:
1. Go to your service
2. Click "Logs" tab
3. Filter by log level if needed

Example log output:
```
2024-01-12T10:30:00Z INFO recommend_a_book_api: Loading configuration...
2024-01-12T10:30:01Z INFO recommend_a_book_api::app: Starting server at http://0.0.0.0:10000
2024-01-12T10:30:05Z INFO recommend_a_book_api::ml: Sentence encoder model loaded successfully
```

## 📈 Monitoring

### Health Monitoring
Render automatically monitors the `/api/health` endpoint and will restart the service if it becomes unhealthy.

### Custom Monitoring
You can monitor:
- Response times via Render metrics
- Error rates in application logs
- Memory/CPU usage in Render dashboard

## 🔄 Deployment Process

### Step-by-Step Instructions

1. **Prepare your code**:
   ```bash
   cd apps/api
   ./scripts/deploy.sh
   ```

2. **Create Render service**:
   - Go to [Render Dashboard](https://dashboard.render.com)
   - Click "New +" → "Web Service"
   - Connect GitHub repository
   - Select your repository

3. **Configure service**:
   - **Name**: `recommend-a-book-rust-api`
   - **Root Directory**: `apps/api`
   - **Runtime**: Detected automatically (Rust)
   - **Build Command**: `cargo build --release`
   - **Start Command**: `./target/release/recommend-a-book-api`

4. **Set environment variables**:
   Add all variables listed in the Environment Variables section above.

5. **Deploy**:
   Click "Create Web Service" and wait for deployment to complete.

### Updating the Service

To update your deployed service:
1. Push changes to your GitHub repository
2. Render will automatically detect changes and redeploy
3. Monitor deployment in Render dashboard

## 💰 Cost Estimation

### Render Pricing (as of 2024)

- **Starter Plan**: $7/month
  - 512MB RAM, 0.1 CPU
  - Good for development/testing

- **Standard Plan**: $25/month
  - 2GB RAM, 1 CPU
  - Recommended for production

- **Pro Plan**: $85/month
  - 8GB RAM, 4 CPU
  - For high-traffic applications

### Additional Costs

- **Supabase**: Free tier available, paid plans from $25/month
- **Pinecone**: Free tier (1 index), paid plans from $70/month
- **HuggingFace**: Free tier available, inference API paid

## 🆘 Support

If you encounter issues:

1. **Check the logs** in Render dashboard
2. **Verify environment variables** are set correctly
3. **Test locally** first with the same configuration
4. **Review this guide** for common solutions

For additional help:
- [Render Documentation](https://render.com/docs)
- [Rust Deployment Guide](https://render.com/docs/deploy-rust)
- Open an issue in the GitHub repository

---

## ✅ Deployment Checklist

Before deploying, ensure:

- [ ] Code builds successfully locally (`cargo build --release`)
- [ ] All tests pass (`cargo test`)
- [ ] Environment variables are prepared
- [ ] Supabase database is set up with required tables
- [ ] Pinecone index is created and has book data
- [ ] GitHub repository is up to date
- [ ] Render service is configured correctly
- [ ] Health check endpoint works after deployment
- [ ] API endpoints return expected responses

🎉 **Congratulations!** Your Rust API should now be running on Render!
