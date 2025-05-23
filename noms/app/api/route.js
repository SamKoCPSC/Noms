import { NextResponse } from "next/server";

export async function GET() {
    return NextResponse.json({
        hello: 'world',
    })
}

export async function POST(request) {
    const data = await request.json()
    return NextResponse.json({
        data,
    })
}