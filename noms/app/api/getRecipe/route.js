import axios from "axios";

export async function GET(req, res) {
    const recipeID = req.nextUrl.searchParams.get('id');

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
                    r.notes,
                    u.name AS author,
                    (
                        SELECT json_agg(
                            json_build_object(
                                'id', i.id,
                                'name', i.name,
                                'quantity', ri.quantity,
                                'unit', ri.unit
                            )
                        )
                        FROM recipe_ingredients ri
                        JOIN ingredients i ON ri.ingredientid = i.id
                        WHERE ri.recipeid = r.id
                    ) AS ingredients,
                    (
                        SELECT json_agg(
                            json_build_object(
                                'branchid', rb.branchid,
                                'branchname', b.name,
                                'position', rb.position,
                                'created_at', rb.created_at
                            ) ORDER BY rb.created_at
                        )
                        FROM recipe_branches rb
                        JOIN branches b ON rb.branchid = b.id
                        WHERE rb.recipeid = r.id
                    ) AS branches
                FROM recipes r
                LEFT JOIN users u ON r.userid = u.id
                WHERE r.id = %s
                GROUP BY r.id, u.name;
            `,
            values: [recipeID]
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
            {status: response.status}
        );
    })
    .catch((error) => {
        return Response.json(
            error.response.data,
            {status: error.response.status}
        );
    });
}
