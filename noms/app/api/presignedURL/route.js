import { PutObjectCommand, S3Client } from "@aws-sdk/client-s3";
import { getSignedUrl } from "@aws-sdk/s3-request-presigner";
import { getServerSession } from "next-auth";
import { authOptions } from "../auth/[...nextauth]/route";

export async function GET(req, res) {
    const session = await getServerSession(authOptions)
    const recipeName = req.nextUrl.searchParams.get('recipeName')
    const fileNames = req.nextUrl.searchParams.getAll('fileNames[]')
    const date = new Date()
    const baseURL = session.user.email + '/' + recipeName + '/' + date.getFullYear() + '-' + (date.getMonth()+1) + '-' + date.getDate() + '_' + date.getHours() + '-' + date.getMinutes() + '-' + date.getSeconds() + '-' + date.getMilliseconds()
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
    const putObjectCommands = fileNames.map((fileName) => {
        const putObjectCommand = new PutObjectCommand({
            Bucket: process.env.S3_BUCKET,
            Key: baseURL + '/' + fileName,
        })
        return putObjectCommand
    })
    return Promise.all(putObjectCommands.map((putObjectCommand) => {
        return getSignedUrl(
            s3Client,
            putObjectCommand,
            {expiresIn: 4}
        )
    })).then((responseURLs) => {
        return Response.json(
            {presignedURLs: responseURLs, baseURL: baseURL},
            {status: 200}
        )
    }).catch(() => {
        return Response.json(
            {message: 'presignedURL failed'},
            {status: 500}
        )
    })
} 