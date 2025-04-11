import axios from "axios";

export async function GET(req, res) {
    const name = req.nextUrl.searchParams.get('name') ? `%${req.nextUrl.searchParams.get('name')}%` : '%%'
    const includedIngredients = ['%flour%', '%sugar%']
    const excludedIngredients = ['%butter%']
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
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
                JOIN recipe_ingredients ri ON r.id = ri.recipeid
				JOIN ingredients i ON i.id = ri.ingredientid
                LEFT JOIN users u ON r.userid = u.id
                LEFT JOIN recipe_ingredients ON r.id = ri.recipeid
                LEFT JOIN ingredients ON ri.ingredientid = i.id
                WHERE r.name ILIKE %s and i.name ILIKE ANY (ARRAY[${'%s,'.repeat(includedIngredients.length).slice(0, -1)}]) 
                AND r.id NOT IN (
                    SELECT r2.id
                    FROM recipes r2
                    JOIN recipe_ingredients ri2 ON r2.id = ri2.recipeid
                    JOIN ingredients i2 ON ri2.ingredientid = i2.id
                    WHERE i2.name ILIKE ANY (ARRAY[${'%s,'.repeat(excludedIngredients.length).slice(0, -1)}])
                )
                GROUP BY r.id, u.name;
            `,
            values: [name].concat(includedIngredients).concat(excludedIngredients)
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