import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { getServerSession } from "next-auth";
import { authOptions } from "../auth/[...nextauth]/route";

export async function GET(req, res) {
    const session = await getServerSession(authOptions)
    const recipeName = req.nextUrl.searchParams.get('recipeName')
    const fileName = req.nextUrl.searchParams.get('fileName')
    const date = new Date()
    const baseURL = session.user.email + '/' + recipeName + '/' + date.getFullYear() + '-' + (date.getMonth()+1) + '-' + date.getDate() + '_' + date.getHours() + '-' + date.getMinutes() + '-' + date.getSeconds() + '-' + date.getMilliseconds() + '-' + fileName
    if(!session) {
        return Response.json(
            {message: 'Missing authentication, you must be logged in'},
            {status: 401}
        )
    }
    const s3Client = new S3Client({
        credentials: {
            accessKeyId: process.env.S3_ACCESS,
            secretAccessKey: process.env.S3_SECRET,
        },
        region: process.env.S3_REGION
    })
    const putObjectCommand = new PutObjectCommand({
        Bucket: process.env.S3_BUCKET,
        Key: baseURL,
    })
    return getSignedUrl(
        s3Client, 
        putObjectCommand, 
        {expiresIn: 4}
    ).then((responseURL) => {
        return Response.json(
            {presignedURL: responseURL, baseURL: baseURL},
            {status: 200}
        )
    }).catch((error) => {
        return Response.json(
            error.response.data,
            {status: error.response.status}
        )
    })
} 