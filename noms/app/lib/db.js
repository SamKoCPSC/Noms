import { Pool } from 'pg';
import fs from 'fs';
import path from 'path';

// Create a connection pool using the RDS environment variables
const pool = new Pool({
  user: process.env.RDS_USER,
  host: process.env.RDS_HOST,
  database: process.env.RDS_NAME,
  password: process.env.RDS_PASSWORD,
  port: parseInt(process.env.RDS_PORT || '5432'),
  ssl: {
    rejectUnauthorized: true,
    ca: fs.readFileSync(path.join(process.cwd(), 'global-bundle.pem')).toString()
  }
});

// Export a helper function to run queries
export async function dbQuery(text, params) {
  const client = await pool.connect();
  try {
    const result = await client.query(text, params);
    return result;
  } catch (error) {
    console.error('Error executing query:', error);
    throw error;
  } finally {
    client.release();
  }
}

// Export the pool in case it's needed elsewhere
export default pool;