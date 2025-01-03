import axios from "axios"
import { getServerSession } from "next-auth";
import { authOptions } from "../auth/[...nextauth]/route";

export async function POST(req, res) {
    const session = await getServerSession(authOptions)
    const data = await req.json()
    const name = data.name
    const description = data.description
    const instructions = JSON.stringify(data.instructions)
    const additionalInfo = JSON.stringify(data.additionalInfo)
    const imageUrls = data.imageUrls
    const status = 'public'
    const ingredients = data.ingredients
    const ingredientNames = ingredients.map(i => `'${i.name}'`).join(", ")
    const ingredientNamesWithBrackets = ingredients.map(i => `('${i.name}')`).join(", ")
    const ingredientQuantitiesCase = ingredients.map(i => `WHEN '${i.name}' THEN ${i.quantity}`).join(" ");
    const ingredientUnitsCase = ingredients.map(i => `WHEN '${i.name}' THEN '${i.unit}'`).join(" ");
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: `
                WITH newRecipe AS (
                    INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status)
                    VALUES (%s, %s, %s, (SELECT id FROM users WHERE email=%s LIMIT 1), %s, %s, %s)
                    RETURNING id
                ),
                existingIngredients AS (
                    SELECT id, name FROM ingredients
                    WHERE name IN (${ingredientNames})
                ),
                newIngredients AS (
                    INSERT INTO ingredients (name)
                    SELECT name FROM (VALUES ${ingredientNamesWithBrackets}) AS new_items(name)
                    WHERE NOT EXISTS (
                        SELECT 1 FROM ingredients
                        WHERE ingredients.name = new_items.name
                    )
                    RETURNING id, name
                ),
                allIngredients AS (
                    SELECT id, name FROM existingIngredients
                    UNION ALL
                    SELECT id, name FROM newIngredients
                )
                INSERT INTO recipe_ingredients (recipeId, ingredientId, quantity, unit)
                SELECT 
                    newRecipe.id, 
                    allIngredients.id, 
                    CASE allIngredients.name 
                        ${ingredientQuantitiesCase}
                    END AS quantity,
                    CASE allIngredients.name 
                        ${ingredientUnitsCase}
                    END AS unit
                FROM newRecipe, allIngredients
            `,
            values: [name, description, instructions, session.user.email, additionalInfo, imageUrls, status]
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