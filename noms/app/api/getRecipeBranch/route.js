import axios from "axios";

export async function GET(req, res) {
    const branchbase = req.nextUrl.searchParams.get('branchbase')
    const branchid = req.nextUrl.searchParams.get('branchid')
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql:
            `
                SELECT * 
                FROM recipes
                WHERE branchbase = %s AND branchid = %s OR id = %s
                ORDER BY id ASC
            `
            ,
            values: [branchbase, branchid, branchbase]
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