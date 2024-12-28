import axios from "axios";

export async function POST(req, res) {
    const data = await req.json()
    const numOfResults = data.numOfResults
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                WITH random_recipes AS (
                    SELECT *
                    FROM recipes
                    WHERE status = 'public'
                    ORDER BY RANDOM()
                    LIMIT %s
                )
                SELECT 
                    r.*, 
                    json_agg(json_build_object('id', i.id, 'name', i.name, 'quantity', ri.quantity, 'unit', ri.unit)) AS ingredients
                FROM random_recipes r
                LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
                LEFT JOIN ingredients i ON ri.ingredientid = i.id
                GROUP BY r.id, r.name, r.datecreated, r.description, r.instructions, r.userid, r.additionalInfo, r.imageURLs, r.status
            `,
            values: [numOfResults]
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