# Nocodo Services API Deployment Guide

## Overview
This document provides instructions for deploying the Nocodo Services API with Nginx reverse proxy and SSL certificates.

## Prerequisites
- Ubuntu/Debian based Linux system
- Root access or sudo privileges
- Domain name `api.nocodo.com` pointing to this server's IP address
- Nginx and Certbot installed

## Deployment Steps

### 1. Install Required Packages (if not already installed)
```bash
sudo apt update
sudo apt install nginx certbot python3-certbot-nginx
```

### 2. Deploy Configuration Files

First, copy the configuration files to the appropriate locations:

```bash
# Create necessary directories
sudo mkdir -p /etc/nginx/sites-available
sudo mkdir -p /etc/nginx/sites-enabled
sudo mkdir -p /var/log/nocodo-services

# Copy Nginx configuration
sudo cp deployment/config/nginx.api.nocodo.com.conf /etc/nginx/sites-available/api.nocodo.com

# Enable the site
sudo ln -sf /etc/nginx/sites-available/api.nocodo.com /etc/nginx/sites-enabled/

# Copy systemd service file
sudo cp deployment/config/nocodo-services.service /etc/systemd/system/nocodo-services.service
```

### 3. Configure Systemd Service

```bash
# Reload systemd to recognize the new service
sudo systemctl daemon-reload

# Enable the service to start on boot
sudo systemctl enable nocodo-services.service

# Start the service
sudo systemctl start nocodo-services.service

# Check service status
sudo systemctl status nocodo-services.service
```

### 4. Generate SSL Certificate

Run the following command to obtain SSL certificate (requires port 80 to be accessible):

```bash
sudo certbot --nginx -d api.nocodo.com
```

Or if using standalone mode:

```bash
sudo certbot certonly --standalone -d api.nocodo.com --agree-tos --email admin@nocodo.com --non-interactive
```

### 5. Test Nginx Configuration

```bash
sudo nginx -t
sudo systemctl reload nginx
```

## Configuration Files

### Nginx Configuration (`/etc/nginx/sites-available/api.nocodo.com`)
The configuration includes:
- HTTP to HTTPS redirect
- SSL termination with proper security headers
- Reverse proxy to the internal API server on port 8080
- Health check endpoint

### Systemd Service (`/etc/systemd/system/nocodo-services.service`)
The service file manages:
- Running the nocodo-services binary
- Restarting on failure
- Proper user/group permissions
- Log management

## Service Management Commands

```bash
# Start service
sudo systemctl start nocodo-services.service

# Stop service
sudo systemctl stop nocodo-services.service

# Restart service
sudo systemctl restart nocodo-services.service

# Check service status
sudo systemctl status nocodo-services.service

# View service logs
sudo journalctl -u nocodo-services.service -f
```

## Verification

1. Verify the service is running:
   ```bash
   sudo systemctl status nocodo-services.service
   ```

2. Verify Nginx configuration:
   ```bash
   sudo nginx -t
   ```

3. Test the API endpoint:
   ```bash
   curl -k https://api.nocodo.com/health
   ```