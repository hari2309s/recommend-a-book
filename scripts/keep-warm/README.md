# API Keep-Warm System

This directory contains tools to prevent cold starts on your API by regularly pinging its endpoints.

## Background: Why We Need This

Serverless deployments like Render.com often "spin down" inactive services to save resources. When a new request comes in after a period of inactivity, the service needs to "cold start" - a process that can take several seconds or even minutes.

This keep-warm system ensures your API stays warm by sending regular pings, preventing users from experiencing these cold start delays.

## Contents

- `keep-warm.sh`: The main script that pings your API endpoints
- `start-background.sh`: Helper script to run keep-warm in the background using nohup
- `crontab-example.txt`: Example crontab configuration for scheduling
- `keep-warm.service`: Systemd service file for running as a system service

## How to Use

### Option 1: Manual/One-off Execution

To run the script once to warm up your API:

```bash
# From project root
./scripts/keep-warm/keep-warm.sh https://recommend-a-book-api.onrender.com

# Or from the scripts/keep-warm directory
cd scripts/keep-warm
./keep-warm.sh
```

### Option 2: GitHub Actions Workflow (Recommended for Most Users)

We've set up a GitHub Actions workflow that automatically pings your API hourly to keep it warm. This is a **free**, reliable solution that doesn't require any server setup.

```bash
# The workflow is already configured in:
.github/workflows/keep-warm.yml
```

Benefits:
- Completely free (GitHub Actions provides 2,000 minutes/month on free plan)
- Runs from GitHub's infrastructure (external pings are more reliable)
- No server configuration needed
- Easy to monitor via GitHub Actions dashboard
- Configured to retry on failures

To use this method:
1. Make sure your repository is on GitHub
2. The workflow will automatically run hourly
3. You can manually trigger it from GitHub Actions tab for testing
4. Review the workflow logs to confirm it's working

### Option 3: Simple Background Process (For Development)

We've provided a convenient script that uses `nohup` to run the keep-warm process in the background:

```bash
# From project root
./scripts/keep-warm/start-background.sh

# Check status
./scripts/keep-warm/start-background.sh --status

# Stop the background process
./scripts/keep-warm/start-background.sh --stop

# View logs
tail -f scripts/keep-warm/logs/keep-warm.log
```

This method is perfect for:
- Development environments
- Quick testing
- Servers where you don't have root access
- When you need a simple solution without cron/systemd

The process will continue running even if you log out of the server.

### Option 4: Set Up as a Cron Job (Reliable Scheduling)

1. Edit the `crontab-example.txt` file to match your environment:
   - Update the API_URL if needed
   - Set the correct path to your project directory
   - Set the correct path to your logs directory
   - Modify the frequency if needed (default is every 10 minutes)

2. Install the crontab:
   ```bash
   crontab scripts/keep-warm/crontab-example.txt
   ```

   Or add to your existing crontab:
   ```bash
   crontab -e
   ```

3. Verify the crontab is installed:
   ```bash
   crontab -l
   ```

### Option 5: Set Up as a Systemd Service (Recommended for Production Servers)

1. Edit the `keep-warm.service` file:
   - Update the ExecStart path to point to your script location
   - Adjust environment variables if needed

2. Install the service:
   ```bash
   sudo cp scripts/keep-warm/keep-warm.service /etc/systemd/system/
   sudo systemctl daemon-reload
   sudo systemctl enable keep-warm.service
   sudo systemctl start keep-warm.service
   ```

3. Check status:
   ```bash
   sudo systemctl status keep-warm.service
   ```

## Configuration Options

The script can be configured using environment variables:

| Variable | Description | Default |
|----------|-------------|---------|
| API_URL | The base URL of your API | https://recommend-a-book-api.onrender.com |
| PING_INTERVAL | Time between pings in seconds | 600 (10 minutes) |
| MAX_RETRIES | Maximum number of retry attempts per ping | 3 |
| RETRY_DELAY | Delay between retries in seconds | 5 |
| LOG_FILE | Path to the log file | ./keep-warm.log |
| LOG_DIR | Directory for log files (used by start-background.sh) | ./logs |
| VERBOSE | Set to any value to enable verbose logging | (not set) |

## Monitoring and Troubleshooting

### Logs

The script outputs logs to:
1. Standard output/error
2. A log file specified by LOG_FILE

Check these logs to verify proper operation or diagnose issues:

```bash
tail -f keep-warm.log
```

### Common Issues

1. **Permission Denied**: Make sure the script is executable:
   ```bash
   chmod +x keep-warm.sh
   ```

2. **API Unreachable**: Verify your API URL and network connectivity.

3. **Script Not Running**: For cron jobs, check cron logs:
   ```bash
   grep CRON /var/log/syslog
   ```

4. **Service Not Running**: For systemd services, check journal:
   ```bash
   journalctl -u keep-warm.service -f
   ```

## Choosing the Right Method

Here's a quick guide to help you choose the right method for your needs:

| Method | Best For | Pros | Cons |
|--------|----------|------|------|
| **GitHub Actions** | Most users and projects | Free, external pinging, zero setup, good monitoring | Requires GitHub repo, limited to hourly schedule |
| **start-background.sh** | Development, testing, personal use | Easy setup, no root needed, works anywhere | Not automatically restarted if server reboots |
| **Cron Job** | Self-hosted production | Simple, widely available, auto-restarts | Less detailed logging, harder to monitor |
| **Systemd Service** | Enterprise production | Best reliability, auto-restart, detailed logging | Requires root access, Linux-only |

## Recommendations

- **For most applications**, use the GitHub Actions approach (free, reliable, easy to monitor)
- For critical production applications, use the systemd service approach
- For development or quick setup, use the start-background.sh method
- Set the ping interval to 5-10 minutes (300-600 seconds) for self-managed approaches
- Monitor logs periodically to ensure the system is working
- For maximum reliability, use multiple methods (e.g., GitHub Actions + one local method)
- Consider setting up monitoring to alert you if the service stops working