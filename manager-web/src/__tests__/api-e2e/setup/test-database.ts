import { execSync } from 'child_process';
import { existsSync, unlinkSync } from 'fs';
import path from 'path';

/**
 * Test database management utilities for API-only e2e tests
 * Handles SQLite database setup, cleanup, and migrations
 */
export class TestDatabaseManager {
  private dbPath: string;
  private migrationPath: string;

  constructor(dbPath?: string, migrationPath?: string) {
    this.dbPath = dbPath || this.getDefaultDbPath();
    this.migrationPath = migrationPath || this.getDefaultMigrationPath();
  }

  /**
   * Set up a clean test database
   */
  async setupTestDatabase(): Promise<void> {
    console.log('Setting up test database...');

    // Remove existing test database if it exists
    if (existsSync(this.dbPath)) {
      unlinkSync(this.dbPath);
    }

    // Run migrations to create schema
    await this.runMigrations();

    console.log('Test database setup complete');
  }

  /**
   * Clean up test database after tests
   */
  async cleanupTestDatabase(): Promise<void> {
    console.log('Cleaning up test database...');

    if (existsSync(this.dbPath)) {
      unlinkSync(this.dbPath);
    }

    console.log('Test database cleanup complete');
  }

  /**
   * Reset database state between tests
   */
  async resetDatabase(): Promise<void> {
    // For SQLite, we can just recreate the database
    await this.cleanupTestDatabase();
    await this.setupTestDatabase();
  }

  /**
   * Run database migrations
   */
  private async runMigrations(): Promise<void> {
    try {
      // Use the manager's migration command if available
      // For now, we'll assume the manager handles migrations on startup
      console.log('Migrations will be run by manager daemon on startup');
    } catch (error) {
      console.error('Failed to run migrations:', error);
      throw error;
    }
  }

  /**
   * Get database statistics for debugging
   */
  async getDatabaseStats(): Promise<{ size: number; tables: string[] }> {
    if (!existsSync(this.dbPath)) {
      return { size: 0, tables: [] };
    }

    const stats = await import('fs').then(fs => fs.statSync(this.dbPath));
    const size = stats.size;

    // For SQLite, we could query table info, but for now return basic stats
    const tables: string[] = []; // Would need to query sqlite_master table

    return { size, tables };
  }

  /**
   * Backup current database state
   */
  async backupDatabase(): Promise<string> {
    const backupPath = `${this.dbPath}.backup`;
    if (existsSync(this.dbPath)) {
      execSync(`cp ${this.dbPath} ${backupPath}`);
    }
    return backupPath;
  }

  /**
   * Restore database from backup
   */
  async restoreDatabase(backupPath: string): Promise<void> {
    if (existsSync(backupPath)) {
      execSync(`cp ${backupPath} ${this.dbPath}`);
      unlinkSync(backupPath);
    }
  }

  /**
   * Get default database path for testing
   */
  private getDefaultDbPath(): string {
    // Use a test-specific database path
    return process.env.TEST_DATABASE_PATH || '/tmp/nocodo-test.db';
  }

  /**
   * Get default migration path
   */
  private getDefaultMigrationPath(): string {
    // Assume migrations are in the manager crate
    return path.join(process.cwd(), '../manager/migrations');
  }

  /**
   * Get current database path
   */
  getDatabasePath(): string {
    return this.dbPath;
  }
}

// Global test database instance
export const testDatabase = new TestDatabaseManager();
