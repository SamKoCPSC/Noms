import axios from "axios"
import { getServerSession } from "next-auth";
import { authOptions } from "../auth/[...nextauth]/route";
import { revalidatePath } from "next/cache";

export async function POST(req, res) {
    // none = new original
    // baseid + branchid = new version
    // baseid + branchbase = new branch
    // baseid + branchbase + branchid = new branched version
    const session = await getServerSession(authOptions)
    const data = await req.json()
    const name = data.name
    const description = data.description
    const instructions = JSON.stringify(data.instructions)
    const additionalInfo = JSON.stringify(data.additionalInfo)
    const imageUrls = data.imageUrls
    const status = data.status
    const baseid = data.baseid
    const baseidSQL = baseid ? '%s' : `currval('recipes_id_seq')`
    const baseidValue = baseid ? [baseid, baseid] : []
    const branchbase = data.branchbase
    const branchid = data.branchid
    const ingredients = data.ingredients
    const ingredientNames = ingredients.map(i => `'${i.name}'`).join(", ")
    const ingredientNamesWithBrackets = ingredients.map(i => `('${i.name}')`).join(", ")
    const ingredientQuantitiesCase = ingredients.map(i => `WHEN '${i.name}' THEN ${i.quantity}`).join(" ");
    const ingredientUnitsCase = ingredients.map(i => `WHEN '${i.name}' THEN '${i.unit}'`).join(" ");
    return axios.post(
        process.env.LAMBDA_API_URL,
        {
            sql: ingredients.length > 0 ? 
                `
                WITH newRecipe AS (
                    INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status, baseid, version, branchid, branchbase)
                    VALUES (%s, %s, %s, %s, %s, %s, %s, ${baseidSQL}, COALESCE((SELECT MAX(version) + 1 FROM recipes WHERE baseid = ${baseidSQL} ${branchid ? `AND branchid = ${branchid}` : ''} ${branchbase && !branchid ? 'AND 1 = 2' : ''}), 1), ${branchid ? '%s' : `COALESCE((SELECT MAX(branchid) + 1 FROM recipes WHERE branchbase = %s), 0${branchbase ? '+1' : ''})`}, %s)
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
                ` 
                :
                `
                INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status, baseid, version)
                VALUES (%s, %s, %s, %s, %s, %s, %s, ${baseidSQL}, COALESCE((SELECT MAX(version) + 1 FROM recipes WHERE baseid = ${baseidSQL}), 1))
                `,
            values: [name, description, instructions, session.user.id, additionalInfo, imageUrls, status].concat(baseidValue).concat([branchid ? branchid : branchbase, branchbase])
        },
        {
            headers: {
                'Content-Type': 'application/json',
                'x-api-key': process.env.LAMBDA_API_KEY,
            }
        }
    ).then((response) => {
        revalidatePath('/myRecipes/1')
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