import axios from "axios"
import { getServerSession } from "next-auth"
import { authOptions } from "../auth/[...nextauth]/route"
import { revalidatePath } from "next/cache"

export async function POST(req, res) {
  const session = await getServerSession(authOptions)
  const data = await req.json()

  const name = data.name
  const description = data.description
  const instructions = JSON.stringify(data.instructions)
  const additionalInfo = JSON.stringify(data.additionalInfo)
  const imageUrls = data.imageUrls
  const status = data.status
  const notes = data.notes
  const ingredients = data.ingredients

  const branchName = data.branchName?.trim() || "Original"
  const branchDescription = data.branchDescription?.trim() || ""

  const projectName = data.projectName?.trim() || "My Project"
  const projectDescription = data.projectDescription?.trim() || ""

  const ingredientNames = ingredients.map(i => `'${i.name}'`).join(", ")
  const ingredientNamesWithBrackets = ingredients.map(i => `('${i.name}')`).join(", ")
  const ingredientQuantitiesCase = ingredients.map(i => `WHEN '${i.name}' THEN ${i.quantity}`).join(" ")
  const ingredientUnitsCase = ingredients.map(i => `WHEN '${i.name}' THEN '${i.unit}'`).join(" ")

  return axios.post(
    process.env.LAMBDA_API_URL,
    {
      sql: ingredients.length > 0
        ? `
          WITH newRecipe AS (
            INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status, notes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
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
          ),
          insertedIngredients AS (
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
            RETURNING recipeId
          ),
          newProject AS (
            INSERT INTO projects (ownerid, name, description)
            VALUES (%s, %s, %s)
            RETURNING id
          ),
          newBranch AS (
            INSERT INTO branches (name, description, ownerid, baserecipeid, headrecipeid, projectid)
            SELECT %s, %s, %s, nr.id, nr.id, np.id
            FROM newRecipe nr, newProject np
            RETURNING id, baserecipeid
          )
          INSERT INTO recipe_branches (recipeid, branchid, position)
          SELECT nr.id, nb.id, 1
          FROM newRecipe nr, newBranch nb
          RETURNING recipeid
        `
        : `
          WITH newRecipe AS (
            INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status, notes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
          ),
          newProject AS (
            INSERT INTO projects (ownerid, name, description)
            VALUES (%s, %s, %s)
            RETURNING id
          ),
          newBranch AS (
            INSERT INTO branches (name, description, ownerid, baserecipeid, headrecipeid, projectid)
            SELECT %s, %s, %s, nr.id, nr.id, np.id
            FROM newRecipe nr, newProject np
            RETURNING id, baserecipeid
          )
          INSERT INTO recipe_branches (recipeid, branchid, position)
          SELECT nr.id, nb.id, 1
          FROM newRecipe nr, newBranch nb
          RETURNING recipeid
        `,
      values: [
        name,
        description,
        instructions,
        session.user.id,
        additionalInfo,
        imageUrls,
        status,
        notes,
        session.user.id,
        projectName,
        projectDescription,
        branchName,
        branchDescription,
        session.user.id,
      ],
    },
    {
      headers: {
        "Content-Type": "application/json",
        "x-api-key": process.env.LAMBDA_API_KEY,
      },
    }
  )
    .then((response) => {
      revalidatePath(`/myRecipes/${session.user.id}`)
      revalidatePath(`/recipe/${response.data.result[0].recipeid}`)

      return Response.json(response.data, { status: response.status })
    })
    .catch((error) => {
      return Response.json(error.response.data, { status: error.response.status })
    })
}
