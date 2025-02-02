import { NextResponse } from "next/server";

export async function GET() {
    console.log("Environment Variables:", process.env);
    return NextResponse.json({
        apiUrl: process.env.LAMBDA_API_URL,
    })
}