import axios from "axios";

export async function GET(req) {
    const { searchParams } = new URL(req.url);
    const branchid = searchParams.get("branchid");

    return axios
        .post(
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
                        b.id AS branchid,
                        b.name AS branchname,
                        rb.position,
                        rb.created_at,
                        json_agg(
                            json_build_object(
                                'id', i.id,
                                'name', i.name,
                                'quantity', ri.quantity,
                                'unit', ri.unit
                            )
                        ) AS ingredients
                    FROM recipe_branches rb
                    INNER JOIN recipes r ON rb.recipeid = r.id
                    INNER JOIN branches b ON rb.branchid = b.id
                    LEFT JOIN users u ON r.userid = u.id
                    LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
                    LEFT JOIN ingredients i ON ri.ingredientid = i.id
                    WHERE rb.branchid = %s
                    GROUP BY r.id, u.name, b.id, rb.position, rb.created_at
                    ORDER BY rb.position ASC, rb.created_at ASC
                `,
                values: [branchid]
            },
            {
                headers: {
                    "Content-Type": "application/json",
                    "x-api-key": process.env.LAMBDA_API_KEY,
                },
            }
        )
        .then((response) => {
            return Response.json(response.data, { status: response.status });
        })
        .catch((error) => {
            return Response.json(error.response?.data || { message: "Error" }, {
                status: error.response?.status || 500,
            });
        });
}
