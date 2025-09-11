import axios from "axios";
import { getServerSession } from "next-auth";
import { authOptions } from "../../auth/[...nextauth]/route";
import { revalidatePath } from "next/cache";

export async function POST(req, { params }) {
  const session = await getServerSession(authOptions);
  const data = await req.json();
  const { branchId } = params;

  const name = data.name;
  const description = data.description;
  const instructions = JSON.stringify(data.instructions);
  const additionalInfo = JSON.stringify(data.additionalInfo);
  const imageUrls = data.imageUrls;
  const status = data.status;
  const notes = data.notes;
  const ingredients = data.ingredients;

  const ingredientNames = ingredients.map(i => `'${i.name}'`).join(", ");
  const ingredientNamesWithBrackets = ingredients.map(i => `('${i.name}')`).join(", ");
  const ingredientQuantitiesCase = ingredients.map(i => `WHEN '${i.name}' THEN ${i.quantity}`).join(" ");
  const ingredientUnitsCase = ingredients.map(i => `WHEN '${i.name}' THEN '${i.unit}'`).join(" ");

  return axios.post(
    process.env.LAMBDA_API_URL,
    {
      sql: ingredients.length > 0
        ? `
          WITH branchInfo AS (
            SELECT headRecipeId
            FROM branches
            WHERE id = %s AND ownerId = %s
          ),
          newRecipe AS (
            INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status, notes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
          ),
          newBranchLink AS (
            INSERT INTO recipe_branches (recipeId, branchId, position, created_at)
            SELECT nr.id, %s,
                   COALESCE((SELECT MAX(position) + 1 FROM recipe_branches WHERE branchId = %s), 1),
                   NOW()
            FROM newRecipe nr
            RETURNING recipeId
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
              nr.id, 
              ai.id, 
              CASE ai.name 
                ${ingredientQuantitiesCase}
              END AS quantity,
              CASE ai.name 
                ${ingredientUnitsCase}
              END AS unit
            FROM newRecipe nr, allIngredients ai
            RETURNING recipeId
          ),
          newParent AS (
            INSERT INTO recipe_parents (recipeId, parentRecipeId)
            SELECT nr.id, bi.headRecipeId
            FROM newRecipe nr, branchInfo bi
            RETURNING recipeId
          )
          UPDATE branches
          SET headRecipeId = nr.id
          FROM newRecipe nr
          WHERE branches.id = %s AND branches.ownerId = %s
          RETURNING nr.id as recipeId, branches.id as branchId, branches.projectId as projectId;
        `
        : `
          WITH branchInfo AS (
            SELECT headRecipeId
            FROM branches
            WHERE id = %s AND ownerId = %s
          ),
          newRecipe AS (
            INSERT INTO recipes (name, description, instructions, userid, additionalInfo, imageurls, status, notes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
          ),
          newBranchLink AS (
            INSERT INTO recipe_branches (recipeId, branchId, position, created_at)
            SELECT nr.id, %s,
                   COALESCE((SELECT MAX(position) + 1 FROM recipe_branches WHERE branchId = %s), 1),
                   NOW()
            FROM newRecipe nr
            RETURNING recipeId
          ),
          newParent AS (
            INSERT INTO recipe_parents (recipeId, parentRecipeId)
            SELECT nr.id, bi.headRecipeId
            FROM newRecipe nr, branchInfo bi
            RETURNING recipeId
          )
          UPDATE branches
          SET headRecipeId = nr.id
          FROM newRecipe nr
          WHERE branches.id = %s AND branches.ownerId = %s
          RETURNING nr.id as recipeId, branches.id as branchId, branches.projectId as projectId;
        `,
      values: [
        branchId,
        session.user.id,
        name,
        description,
        instructions,
        session.user.id,
        additionalInfo,
        imageUrls,
        status,
        notes,
        branchId,
        branchId,
        branchId,
        session.user.id
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
      if (!response.data.result || response.data.result.length === 0) {
        return Response.json({ error: "Branch not found" }, { status: 404 });
      }

      revalidatePath(`/myRecipes/${session.user.id}`);
      revalidatePath(`/project/${response.data.result[0].projectid}`);
      revalidatePath(`/branch/${response.data.result[0].branchid}`);
      revalidatePath(`/recipe/${response.data.result[0].recipeid}`);
      return Response.json(response.data, { status: response.status });
    })
    .catch((error) => {
      return Response.json(error.response?.data || { error: error.message }, { status: error.response?.status || 500 });
    });
}
