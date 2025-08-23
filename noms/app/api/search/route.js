import axios from "axios";

export async function GET(req, res) {
    const name = `%${req.nextUrl.searchParams.get('name')}%`
    const includedIngredients = JSON.parse(req.nextUrl.searchParams.get('includedIngredients')).length > 0 ? JSON.parse(req.nextUrl.searchParams.get('includedIngredients')) : ['%']
    const excludedIngredients = JSON.parse(req.nextUrl.searchParams.get('excludedIngredients') || '[]')
    const requiredIngredients = JSON.parse(req.nextUrl.searchParams.get('requiredIngredients') || '[]')
    // Build the SQL query dynamically based on whether requiredIngredients exist
    let sqlQuery = `
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
        WHERE r.name ILIKE %s and i.name ILIKE ANY (ARRAY[${'%s,'.repeat(includedIngredients.length).slice(0, -1)}]) 
    `;

    let values = [name].concat(includedIngredients);

    // Add required ingredients condition (AND logic)
    if (requiredIngredients.length > 0) {
        sqlQuery += `
            AND r.id IN (
                SELECT r3.id
                FROM recipes r3
                JOIN recipe_ingredients ri3 ON r3.id = ri3.recipeid
                JOIN ingredients i3 ON ri3.ingredientid = i3.id
                WHERE i3.name ILIKE ANY (ARRAY[${'%s,'.repeat(requiredIngredients.length).slice(0, -1)}])
                GROUP BY r3.id
                HAVING COUNT(DISTINCT i3.name) >= ${requiredIngredients.length}
            )
        `;
        values = values.concat(requiredIngredients);
    }

    // Add excluded ingredients condition
    if (excludedIngredients.length > 0) {
        sqlQuery += `
            AND r.id NOT IN (
                SELECT r2.id
                FROM recipes r2
                JOIN recipe_ingredients ri2 ON r2.id = ri2.recipeid
                JOIN ingredients i2 ON ri2.ingredientid = i2.id
                WHERE i2.name ILIKE ANY (ARRAY[${'%s,'.repeat(excludedIngredients.length).slice(0, -1)}]::TEXT[])
            )
        `;
        values = values.concat(excludedIngredients);
    }

    sqlQuery += `
        GROUP BY r.id, u.name;
    `;

    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: sqlQuery,
            values: values
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