# Nocodo Services API Deployment with Ansible

## Overview
This document provides instructions for deploying the Nocodo Services API using Ansible for automated deployment and configuration management.

## Prerequisites
- Ansible installed on control machine
- SSH access to target server with key-based authentication
- Target server running Ubuntu/Debian
- Domain name `api.nocodo.com` pointing to target server's IP address

## Environment Variables
Set these environment variables before running the playbook:

```bash
export NOCODO_SERVER_IP="your-server-ip-address"
export NOCODO_SSH_USER="your-ssh-username"
export NOCODO_SSH_KEY_PATH="/path/to/your/private/key"
export NOCODO_DOMAIN="api.nocodo.com"
```

## Directory Structure
```
ansible/
├── inventory.ini
├── group_vars/all.yml
├── roles/
│   ├── nginx/
│   │   ├── tasks/main.yml
│   │   └── templates/nginx.api.nocodo.com.conf.j2
│   ├── service/
│   │   ├── tasks/main.yml
│   │   └── templates/nocodo-services.service.j2
│   └── ssl/
│       ├── tasks/main.yml
│       └── handlers/main.yml
├── deploy.yml
└── status.yml
```

## Deployment Playbook (`deploy.yml`)
```yaml
---
- name: Deploy Nocodo Services API
  hosts: nocodo_servers
  become: yes
  vars_files:
    - group_vars/all.yml
  
  tasks:
    - name: Install required packages
      apt:
        name:
          - nginx
          - certbot
          - python3-certbot-nginx
        update_cache: yes

    - name: Create log directory
      file:
        path: /var/log/nocodo-services
        state: directory
        owner: root
        group: root
        mode: '0755'

    - name: Copy Nginx configuration
      template:
        src: nginx.api.nocodo.com.conf.j2
        dest: /etc/nginx/sites-available/api.nocodo.com
        mode: '0644'
        owner: root
        group: root

    - name: Enable Nginx site
      file:
        src: /etc/nginx/sites-available/api.nocodo.com
        dest: /etc/nginx/sites-enabled/api.nocodo.com
        state: link

    - name: Copy systemd service file
      template:
        src: nocodo-services.service.j2
        dest: /etc/systemd/system/nocodo-services.service
        mode: '0644'
        owner: root
        group: root

    - name: Reload systemd
      systemd:
        daemon_reload: yes

    - name: Enable and start service
      systemd:
        name: nocodo-services.service
        enabled: yes
        state: started

    - name: Test Nginx configuration
      command: nginx -t
      changed_when: no

    - name: Reload Nginx
      systemd:
        name: nginx
        state: reloaded

    - name: Obtain SSL certificate
      command: >
        certbot --nginx -d {{ nocodo_domain }} --agree-tos --email admin@{{ nocodo_domain }} --non-interactive
      args:
        creates: /etc/letsencrypt/live/{{ nocodo_domain }}/fullchain.pem
      when: ssl_enabled | default(true)
```

## Status Check Playbook (`status.yml`)
```yaml
---
- name: Check Nocodo Services API Status
  hosts: nocodo_servers
  become: yes
  gather_facts: yes
  
  tasks:
    - name: Check service status
      command: systemctl is-active nocodo-services.service
      register: service_status
      changed_when: false

    - name: Display service status
      debug:
        msg: "Service is {{ 'running' if service_status.stdout == 'active' else 'not running' }}"

    - name: Check Nginx configuration
      command: nginx -t
      register: nginx_check
      changed_when: false
      ignore_errors: yes

    - name: Display Nginx status
      debug:
        msg: "Nginx configuration is {{ 'valid' if nginx_check.rc == 0 else 'invalid' }}"

    - name: Check SSL certificate
      stat:
        path: /etc/letsencrypt/live/{{ nocodo_domain }}/fullchain.pem
      register: ssl_cert
      changed_when: false

    - name: Display SSL status
      debug:
        msg: "SSL certificate {{ 'found' if ssl_cert.stat.exists else 'not found' }}"
```

## Inventory File (`inventory.ini`)
```ini
[nocodo_servers]
{{ nocodo_server_ip }} ansible_user={{ nocodo_ssh_user }} ansible_ssh_private_key_file={{ nocodo_ssh_key_path }}
```

## Group Variables (`group_vars/all.yml`)
```yaml
---
nocodo_domain: "{{ lookup('env', 'NOCODO_DOMAIN') | default('api.nocodo.com') }}"
nocodo_server_ip: "{{ lookup('env', 'NOCODO_SERVER_IP') }}"
nocodo_ssh_user: "{{ lookup('env', 'NOCODO_SSH_USER') }}"
nocodo_ssh_key_path: "{{ lookup('env', 'NOCODO_SSH_KEY_PATH') }}"
ssl_enabled: true
```

## Usage Instructions

### 1. Set Environment Variables
```bash
export NOCODO_SERVER_IP="your-server-ip-address"
export NOCODO_SSH_USER="your-ssh-username"
export NOCODO_SSH_KEY_PATH="/path/to/your/private/key"
export NOCODO_DOMAIN="api.nocodo.com"
```

### 2. Deploy the Application
```bash
ansible-playbook -i inventory.ini deploy.yml
```

### 3. Check Status
```bash
ansible-playbook -i inventory.ini status.yml
```

## Configuration Templates

### Nginx Configuration Template (`roles/nginx/templates/nginx.api.nocodo.com.conf.j2`)
```nginx
server {
    listen 80;
    server_name {{ nocodo_domain }};

    # Redirect all HTTP requests to HTTPS
    return 301 https://$server_name$request_uri;
}

server {
    listen 443 ssl http2;
    server_name {{ nocodo_domain }};

    ssl_certificate /etc/letsencrypt/live/{{ nocodo_domain }}/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/{{ nocodo_domain }}/privkey.pem;
    ssl_trusted_certificate /etc/letsencrypt/live/{{ nocodo_domain }}/chain.pem;

    # SSL Configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers ECDHE-RSA-AES256-GCM-SHA512:DHE-RSA-AES256-GCM-SHA512:ECDHE-RSA-AES256-GCM-SHA384:DHE-RSA-AES256-GCM-SHA384;
    ssl_prefer_server_ciphers off;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;

    # Security Headers
    add_header X-Frame-Options "SAMEORIGIN" always;
    add_header X-XSS-Protection "1; mode=block" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header Referrer-Policy "no-referrer-when-downgrade" always;
    add_header Content-Security-Policy "default-src * data: blob: filesystem: about: ws: wss: 'unsafe-inline' 'unsafe-eval'" always;

    # Proxy settings
    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_set_header X-Forwarded-Host $server_name;
        
        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 30s;
    }

    # Health check endpoint
    location /health {
        access_log off;
        return 200 "healthy\n";
        add_header Content-Type text/plain;
    }
}
```

### Systemd Service Template (`roles/service/templates/nocodo-services.service.j2`)
```ini
[Unit]
Description=Nocodo Services API
After=network.target

[Service]
Type=simple
User=nocodo
Group=nocodo
Restart=always
RestartSec=10
Environment=NODE_ENV=production
ExecStart=/home/nocodo/Projects/nocodoWorktrees/issue-92-Initialize_Services_API/target/release/nocodo-services
WorkingDirectory=/home/nocodo/Projects/nocodoWorktrees/issue-92-Initialize_Services_API
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
```

## Service Management Commands

After deployment, you can manage the service using standard systemctl commands:

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
   curl -k https://{{ nocodo_domain }}/health
   ```