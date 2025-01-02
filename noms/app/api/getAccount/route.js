import axios from "axios";

export async function GET(req, res) {
    const userID = req.nextUrl.searchParams.get('id')
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                SELECT * 
                FROM users 
                WHERE id = %s;
            `,
            values: [userID]
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