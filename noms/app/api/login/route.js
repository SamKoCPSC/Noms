import axios from "axios";

export async function POST(req, res) {
    const data = await req.json()
    const name = data.name
    const email = data.email

    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
            INSERT INTO users (name, email)
            VALUES (%s, %s)
            ON CONFLICT (email) DO NOTHING
            `,
            values: [name, email]
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