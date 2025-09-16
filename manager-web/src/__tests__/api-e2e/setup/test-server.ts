import { ChildProcess, spawn } from 'child_process';
import { testApiClient } from './api-client';

/**
 * Test server management utilities for API-only e2e tests
 * Handles lifecycle of the manager daemon during testing
 */
export class TestServerManager {
  private serverProcess: ChildProcess | null = null;
  private serverReady = false;
  private readonly port = 8081;
  private readonly host = 'http://localhost';
  private readonly baseURL = `${this.host}:${this.port}`;

  /**
   * Start the manager daemon server for testing
   */
  async startServer(configPath?: string): Promise<void> {
    if (this.serverProcess) {
      throw new Error('Server is already running');
    }

    const config = configPath || this.getDefaultConfigPath();

    return new Promise((resolve, reject) => {
      console.log('Starting manager daemon for API tests...');

      // Build the command to start the manager daemon
      const command = '../target/release/nocodo-manager';
      const args = ['--config', 'manager/test-config.toml'];

      this.serverProcess = spawn(command, args, {
        stdio: ['pipe', 'pipe', 'pipe'],
        env: { ...process.env, RUST_LOG: 'info' },
        cwd: process.cwd(),
      });

      if (!this.serverProcess) {
        reject(new Error('Failed to spawn server process'));
        return;
      }

      // Set up output handling
      this.serverProcess.stdout?.on('data', (data) => {
        const output = data.toString();
        console.log(`[SERVER] ${output.trim()}`);

        // Check for server ready indicator
        if (output.includes('listening on') || output.includes('Server started')) {
          this.serverReady = true;
        }
      });

      this.serverProcess.stderr?.on('data', (data) => {
        const output = data.toString();
        console.error(`[SERVER ERROR] ${output.trim()}`);
      });

      this.serverProcess.on('error', (error) => {
        console.error('Server process error:', error);
        reject(error);
      });

      this.serverProcess.on('exit', (code, signal) => {
        console.log(`Server process exited with code ${code}, signal ${signal}`);
        this.serverProcess = null;
        this.serverReady = false;
      });

      // Wait for server to be ready
      this.waitForServerReady()
        .then(() => {
          console.log('Manager daemon is ready for testing');
          resolve();
        })
        .catch(reject);
    });
  }

  /**
   * Stop the manager daemon server
   */
  async stopServer(): Promise<void> {
    if (!this.serverProcess) {
      return;
    }

    return new Promise((resolve) => {
      console.log('Stopping manager daemon...');

      // Send SIGTERM first
      this.serverProcess!.kill('SIGTERM');

      // Wait for graceful shutdown or force kill after timeout
      const timeout = setTimeout(() => {
        if (this.serverProcess) {
          console.log('Force killing server process...');
          this.serverProcess.kill('SIGKILL');
        }
      }, 5000);

      this.serverProcess.on('exit', () => {
        clearTimeout(timeout);
        this.serverProcess = null;
        this.serverReady = false;
        console.log('Manager daemon stopped');
        resolve();
      });
    });
  }

  /**
   * Wait for the server to be ready by polling health endpoint
   */
  private async waitForServerReady(maxAttempts = 30, intervalMs = 1000): Promise<void> {
    for (let attempt = 1; attempt <= maxAttempts; attempt++) {
      try {
        const client = new testApiClient.constructor(this.baseURL) as typeof testApiClient;
        await client.healthCheck();
        return;
      } catch (error) {
        console.log(`Waiting for server... (attempt ${attempt}/${maxAttempts})`);
        await new Promise(resolve => setTimeout(resolve, intervalMs));
      }
    }
    throw new Error('Server failed to start within timeout');
  }

  /**
   * Check if server is currently running
   */
  isServerRunning(): boolean {
    return this.serverProcess !== null && !this.serverProcess.killed;
  }

  /**
   * Get the base URL for the test server
   */
  getBaseURL(): string {
    return this.baseURL;
  }

  /**
   * Get default config path for testing
   */
  private getDefaultConfigPath(): string {
    // Use a test-specific config or default location
    return process.env.NOCODO_CONFIG || '~/.config/nocodo/manager.toml';
  }

  /**
   * Reset server state between tests
   */
  async resetServer(): Promise<void> {
    // This could involve database cleanup, cache clearing, etc.
    // For now, just ensure server is running
    if (!this.isServerRunning()) {
      await this.startServer();
    }
  }
}

// Global test server instance
export const testServer = new TestServerManager();