import axios from "axios";

export async function GET(req, res) {
    const variantName = `%${req.nextUrl.searchParams.get('variantName')}%` || '';

    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                SELECT 
                    b.id AS branchid,
                    b.name AS variantname,
                    u.name AS ownername,
                    r.name AS latestrecipename,
                    r.imageurls AS latestimageurls
                FROM branches b
                JOIN users u 
                    ON b.ownerid = u.id
                LEFT JOIN recipes r
                    ON b.headrecipeid = r.id
                WHERE b.name ILIKE %s
                ORDER BY b.name ASC;
            `,
            values: [variantName]
        },
        {
            headers: {
                'Content-Type': 'application/json',
                'x-api-key': process.env.LAMBDA_API_KEY,
            }
        }
    )
    .then((response) => {
        return Response.json(
            response.data,
            { status: response.status }
        );
    })
    .catch((error) => {
        return Response.json(
            error.response?.data || { error: "Unknown error" },
            { status: error.response?.status || 500 }
        );
    });
}
