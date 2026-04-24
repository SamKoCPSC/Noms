/** @type {import('next').NextConfig} */
const nextConfig = {
    env: {
        GOOGLE_CLIENT_ID: process.env.GOOGLE_CLIENT_ID,
        GOOGLE_CLIENT_SECRET: process.env.GOOGLE_CLIENT_SECRET,
        LAMBDA_API_KEY: process.env.LAMBDA_API_KEY,
        LAMBDA_API_URL: process.env.LAMBDA_API_URL,
        NEXTAUTH_SECRET: process.env.NEXTAUTH_SECRET,
        NEXTAUTH_URL: process.env.NEXTAUTH_URL,
        NOMS_URL: process.env.NOMS_URL,
        S3_ACCESS: process.env.S3_ACCESS,
        S3_BUCKET: process.env.S3_BUCKET,
        S3_REGION: process.env.S3_REGION,
        S3_SECRET: process.env.S3_SECRET,
        RDS_USER: process.env.RDS_USER,
        RDS_PASSWORD: process.env.RDS_PASSWORD,
        RDS_HOST: process.env.RDS_HOST,
        RDS_PORT: process.env.RDS_PORT,
        RDS_NAME: process.env.RDS_NAME,
      }, 
};

export default nextConfig;