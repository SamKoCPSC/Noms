import axios from "axios";

export async function GET(req, res) {
    const userID = req.nextUrl.searchParams.get('id') || undefined
    const userEmail = req.nextUrl.searchParams.get('email') || undefined
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: userID &&
            `
                SELECT * 
                FROM users 
                WHERE id = %s;
            ` || userEmail &&
            `
                SELECT * 
                FROM users 
                WHERE email = %s;
            ` || !userID && !userEmail &&
            `
                SELECT *
                FROM users
            `
            ,
            values: userID || userEmail ? [userID || userEmail] : []
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