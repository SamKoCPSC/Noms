import axios from "axios"

export async function POST(req, res) {
    const data = await req.json()
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
            
            `,
            values: [name, description, instructions, userId, additionalInfo, imageUrls, status]
        },
        {
            headers: {
                'Content-Type': 'application/json',
                'x-api-key': process.env.LAMBDA_API_KEY,
            }
        }
    ).then((response) => {
        return Response.json(
            response.data,
            {status: response.status}
        )
    }).catch((error) => {
        return Response.json(
            error.response.data,
            {status: error.response.status}
        )
    })
}