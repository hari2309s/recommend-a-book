# Deployment Guide for Recommend-a-Book API

This guide explains how to deploy the Recommend-a-Book API service, with a focus on securely managing configuration and API keys.

## Configuration System

The API uses a layered configuration system:

1. Base configuration (`config/base.toml`) - Common settings for all environments
2. Environment-specific configuration (`config/development.toml`, `config/production.toml`) - Settings specific to each environment
3. Local overrides (`config/local.toml`) - Developer-specific settings that override other configs
4. Environment variables - The highest priority, overriding all file-based configs

## Required API Keys and Credentials

The application requires several external service credentials:

- **Supabase**: Authentication and database access
  - `APP_SUPABASE_URL` - Your Supabase project URL
  - `APP_SUPABASE_KEY` - Your Supabase API key

- **Pinecone**: Vector database for book embeddings
  - `APP_PINECONE_API_KEY` - Your Pinecone API key
  - `APP_PINECONE_ENV` - Pinecone environment (e.g., "gcp-starter" or "us-west1-gcp")
  - `APP_PINECONE_INDEX_NAME` - Name of your Pinecone index

- **HuggingFace** (optional): For machine learning models
  - `APP_HUGGINGFACE_API_KEY` - Your HuggingFace API key

## Local Development Setup

For local development:

1. Copy example config files:
   ```
   cp config/development.toml.example config/development.toml
   cp config/local.toml.example config/local.toml
   ```

2. Edit these files to add your development API keys.

3. **Important**: Never commit these files to Git. They are already in `.gitignore`.

4. Alternatively, you can use environment variables:
   ```
   export APP_SUPABASE_URL="your-supabase-url"
   export APP_SUPABASE_KEY="your-supabase-key"
   export APP_PINECONE_API_KEY="your-pinecone-key"
   export APP_PINECONE_ENV="your-pinecone-env"
   export APP_PINECONE_INDEX_NAME="your-index-name"
   ```

## Deployment to Render

### Setting Up the Deployment

1. Connect your GitHub repository to Render.

2. Create a new Web Service using the existing `render.yaml` configuration.

3. **Critical**: Set all API keys as environment variables in the Render dashboard:
   - `APP_SUPABASE_URL`
   - `APP_SUPABASE_KEY`
   - `APP_PINECONE_API_KEY`
   - `APP_PINECONE_ENVIRONMENT`
   - `APP_PINECONE_INDEX`
   - `APP_HUGGINGFACE_API_KEY` (if needed)

4. Ensure the "sync" setting is disabled for all sensitive environment variables in the Render dashboard.

### Security Best Practices

1. **Never commit API keys to Git**
   - Use example files with placeholders
   - Rely on environment variables for actual deployment

2. **Use different API keys for development and production**
   - Create separate credentials for each environment
   - Restrict production keys to necessary permissions only

3. **Regularly rotate API keys**
   - Update keys periodically for security
   - Update deployment environment variables when keys change

## Troubleshooting

### Common Pinecone Connection Issues

If you encounter an error like this:
```
{
    "error": "Pinecone error: Request failed: error sending request for url (https://your_index_name-project.svc.your_environment_name.pinecone.io/query): error trying to connect: dns error: failed to lookup address information: Name or service not known"
}
```

This indicates that the application is using placeholder values instead of actual Pinecone configuration. Here's how to fix it:

1. **Verify Render Environment Variables**:
   - Go to your Render dashboard → Your service → Environment
   - Ensure the following variables are properly set with actual values (not placeholders):
     - `APP_PINECONE_API_KEY` - Your Pinecone API key
     - `APP_PINECONE_ENV` - The correct Pinecone environment (e.g., "gcp-starter" or "us-west1-gcp")
     - `APP_PINECONE_INDEX_NAME` - The name of your Pinecone index (without the "-project" suffix)

2. **Check Pinecone Dashboard**:
   - Log in to your Pinecone dashboard
   - Verify the correct index name
   - Confirm the environment name (displayed in the URL or index details)
   - Check that your API key is valid

3. **Pinecone URL Format**:
   - For newer Pinecone environments, the URL format is: `https://{index-name}.svc.{environment}.pinecone.io`
   - For legacy environments, it might be: `https://{index-name}-project.svc.{environment}.pinecone.io`
   - Our code handles both formats, but you must provide the correct index name and environment

4. **Restart Your Deployment**:
   - After updating the environment variables, restart your service in Render
   - Check logs for configuration information

5. **Diagnostic Information**:
   - Set `RUST_LOG=info` or `RUST_LOG=debug` in your Render environment variables
   - Review logs for more detailed error information about Pinecone connections

### Authenticating with Pinecone

If you see errors about unauthorized access:

1. Verify your API key is correctly set in the Render dashboard
2. Check that your Pinecone account has access to the specified index
3. Try regenerating your Pinecone API key and updating it in Render

### Other Deployment Issues

If your deployment fails with other errors:

1. Verify all environment variables are correctly set in the Render dashboard
2. Check application logs for any "missing configuration" errors
3. Ensure the API keys have the correct permissions
4. Verify that you haven't inadvertently restricted network access in the external services

## Cold Start Mitigation

When deploying the API on Render's free or starter tiers, you may experience "cold starts" where the application takes a significant time to initialize after periods of inactivity. We've implemented several techniques to address this:

### Prewarming Mechanism

The API includes a dedicated prewarming system:

1. **Prewarm Endpoint**: `/api/prewarm` 
   - Initializes all services (ML model, Pinecone, caches)
   - Can be called manually or automatically
   - Returns status of the prewarming operation

2. **Modified Health Check**:
   - Health check endpoints now trigger background prewarming
   - Helps maintain service readiness without blocking health checks

3. **Optimized Startup**:
   - The `render.yaml` is configured to call the prewarm endpoint on startup
   - Services use lazy initialization to improve startup times
   - Caching is implemented for both Pinecone and ML results

### Keep-Warm Script

A `keep-warm.sh` script is provided to prevent service hibernation:

1. **Usage**:
   ```bash
   # Run locally to keep the remote service warm
   ./keep-warm.sh https://your-api-url.onrender.com
   ```

2. **Features**:
   - Periodically pings the API (default: every 15 minutes)
   - Alternates between prewarm and health endpoints
   - Has built-in retry logic and logging
   - Can be run on any machine to keep the service warm

3. **Setting Up Scheduled Pings**:
   - Use a cron job on a separate system
   - Consider a simple GitHub Action that runs periodically
   - Or use a dedicated service like UptimeRobot or Cronitor

### Configuration Options

The following environment variables control cold start mitigation:

- `APP_ENABLE_PREWARM`: Set to "true" to enable prewarming (default: true)
- `APP_PREWARM_TIMEOUT_SECS`: Maximum time for prewarming (default: 30s)
- `KEEP_WARM_INTERVAL`: Minutes between keep-warm pings (default: 15)

### Best Practices

1. **Initial Deployment**: 
   - Manually trigger the prewarm endpoint after deployment
   - `curl https://your-api-url.onrender.com/api/prewarm`
   - Wait for it to complete before directing traffic to the API

2. **Production Use**:
   - Upgrade to a paid Render tier for consistent performance
   - Set up the keep-warm script on a reliable external system
   - Consider implementing a client-side retry strategy with exponential backoff

3. **Monitoring**:
   - Watch for "Prewarming API services" log entries to track initialization
   - Monitor cold start frequency and duration to optimize settings

## Monitoring and Maintenance

- Regularly check Render logs for any authentication or configuration issues
- Monitor API key usage in the respective service dashboards
- Set up alerts for unusual API usage patterns
- Check Pinecone usage metrics to ensure you're within your plan limits
- Monitor cold start frequency to adjust keep-warm interval if needed

For additional help, contact the development team or file an issue in the repository.