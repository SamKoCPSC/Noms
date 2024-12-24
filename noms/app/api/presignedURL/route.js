import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";

export async function GET(req, res) {
    const s3Client = new S3Client({
        credentials: {
            accessKeyId: process.env.S3_ACCESS,
            secretAccessKey: process.env.S3_SECRET,
        },
        region: process.env.S3_REGION
    })
    const putObjectCommand = new PutObjectCommand({
        Bucket: process.env.S3_BUCKET,
        Key: 'test.jpg',
    })
    return getSignedUrl(
        s3Client, 
        putObjectCommand, 
        {expiresIn: 3600}
    ).then((url) => {
        return Response.json(
            {url: url},
            {status: 200}
        )
    }).catch((error) => {
        return Response.json(
            error.response.data,
            {status: error.response.status}
        )
    })
}