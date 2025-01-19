import axios from "axios";

export async function GET(req, res) {
    const name = `%${req.nextUrl.searchParams.get('name')}%`
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                SELECT *
                FROM recipes
                WHERE name ILIKE %s
            `,
            values: [name]
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