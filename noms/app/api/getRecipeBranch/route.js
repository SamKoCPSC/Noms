import axios from "axios";

export async function GET(req, res) {
    const branchbase = req.nextUrl.searchParams.get('branchbase')
    const branchid = req.nextUrl.searchParams.get('branchid')
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql:
            `
                SELECT 
                    r.id AS recipeid,
                    r.name AS name,
                    r.description,
                    r.instructions,
                    r.userid,
                    r.additionalinfo,
                    r.imageurls,
                    r.status,
                    r.datecreated,
                    r.baseid,
                    r.version,
                    r.branchid,
                    r.branchbase,
                    r.notes,
                    u.name AS author,
                    json_agg(
                        json_build_object(
                            'id', i.id,
                            'name', i.name,
                            'quantity', ri.quantity,
                            'unit', ri.unit
                        )
                    ) AS ingredients
                FROM recipes r
                LEFT JOIN users u ON r.userid = u.id
                LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
                LEFT JOIN ingredients i ON ri.ingredientid = i.id
                WHERE r.branchbase = %s AND r.branchid = %s OR r.id = %s
                GROUP BY r.id, u.name
                ORDER BY r.id ASC
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