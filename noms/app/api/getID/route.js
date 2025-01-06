import axios from "axios";

export async function GET(req, res) {
    const email = req.nextUrl.searchParams.get('email')
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                SELECT id 
                FROM users 
                WHERE email = %s;
            `,
            values: [email]
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