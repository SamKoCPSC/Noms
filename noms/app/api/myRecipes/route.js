import axios from "axios";

export async function GET(req, res) {
    const userID = req.nextUrl.searchParams.get('id')
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                SELECT 
                    r.id AS id,
                    r.name AS name,
                    r.description,
                    r.instructions,
                    r.datecreated,
                    r.additionalinfo,
                    r.imageurls,
                    r.status,
                    u.name AS author,
                    u.email
                FROM 
                    recipes r
                JOIN 
                    users u ON r.userid = u.id
                WHERE 
                    u.id = %s;
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