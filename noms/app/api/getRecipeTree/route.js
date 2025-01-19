import axios from "axios";

export async function GET(req, res) {
    const baseid = req.nextUrl.searchParams.get('baseid')
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
            FROM 
                recipes r
            JOIN 
                (
                    SELECT 
                        branchid,
                        branchbase, 
                        baseid, 
                        MAX(version) AS max_version
                    FROM 
                        recipes
                    WHERE
                        baseid = %s
                    GROUP BY 
                        branchid, branchbase, baseid
                ) latest
            ON 
                r.branchid = latest.branchid 
                AND r.branchbase = latest.branchbase 
                AND r.baseid = latest.baseid 
                AND r.version = latest.max_version
            LEFT JOIN users u ON r.userid = u.id
            LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
            LEFT JOIN ingredients i ON ri.ingredientid = i.id
            GROUP BY r.id, u.name
            ORDER BY r.branchbase ASC, r.branchid ASC
            `
            ,
            values: [baseid]
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