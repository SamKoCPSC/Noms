import axios from "axios";
import { getServerSession } from "next-auth";
import { authOptions } from "../../auth/[...nextauth]/route";
import { revalidatePath } from "next/cache";

export async function POST(req, { params }) {
  const session = await getServerSession(authOptions);
  const data = await req.json();
  const { recipeId } = params; // base recipe id we’re branching from

  const name = data.name;
  const description = data.description;
  const instructions = JSON.stringify(data.instructions);
  const additionalinfo = JSON.stringify(data.additionalInfo);
  const imageurls = data.imageUrls;
  const status = data.status;
  const notes = data.notes;
  const ingredients = data.ingredients;

  // branch metadata
  const branchName = data.branchName;
  const branchDescription = data.branchDescription;

  const ingredientNames = ingredients.map(i => `'${i.name}'`).join(", ");
  const ingredientNamesWithBrackets = ingredients.map(i => `('${i.name}')`).join(", ");
  const ingredientQuantitiesCase = ingredients.map(i => `WHEN '${i.name}' THEN ${i.quantity}`).join(" ");
  const ingredientUnitsCase = ingredients.map(i => `WHEN '${i.name}' THEN '${i.unit}'`).join(" ");

  return axios.post(
    process.env.LAMBDA_API_URL,
    {
      sql: ingredients.length > 0
        ? `
          WITH basebranch AS (
            SELECT rb.branchid, b.projectid
            FROM recipe_branches rb
            JOIN branches b ON rb.branchid = b.id
            WHERE rb.recipeid = %s
            LIMIT 1
          ),
          newrecipe AS (
            INSERT INTO recipes (name, description, instructions, userid, additionalinfo, imageurls, status, notes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
          ),
          newbranch AS (
            INSERT INTO branches (name, description, baserecipeid, headrecipeid, ownerid, projectid)
            SELECT %s, %s, %s, nr.id, %s, bb.projectid
            FROM newrecipe nr, basebranch bb
            RETURNING id, headrecipeid
          ),
          newbranchlink AS (
            INSERT INTO recipe_branches (recipeid, branchid, position, created_at)
            SELECT nr.id, nb.id,
                   COALESCE((SELECT MAX(position) + 1 FROM recipe_branches WHERE branchid = nb.id), 1),
                   NOW()
            FROM newrecipe nr, newbranch nb
            RETURNING recipeid
          ),
          existingingredients AS (
            SELECT id, name FROM ingredients
            WHERE name IN (${ingredientNames})
          ),
          newingredients AS (
            INSERT INTO ingredients (name)
            SELECT name FROM (VALUES ${ingredientNamesWithBrackets}) AS new_items(name)
            WHERE NOT EXISTS (
              SELECT 1 FROM ingredients
              WHERE ingredients.name = new_items.name
            )
            RETURNING id, name
          ),
          allingredients AS (
            SELECT id, name FROM existingingredients
            UNION ALL
            SELECT id, name FROM newingredients
          ),
          insertedingredients AS (
            INSERT INTO recipe_ingredients (recipeid, ingredientid, quantity, unit)
            SELECT 
              nr.id, 
              ai.id, 
              CASE ai.name 
                ${ingredientQuantitiesCase}
              END AS quantity,
              CASE ai.name 
                ${ingredientUnitsCase}
              END AS unit
            FROM newrecipe nr, allingredients ai
            RETURNING recipeid
          ),
          newparent AS (
            INSERT INTO recipe_parents (recipeid, parentrecipeid)
            SELECT nr.id, %s
            FROM newrecipe nr
            RETURNING recipeid
          )
          SELECT nr.id AS recipeid, nb.id AS branchid, bb.projectid
          FROM newrecipe nr, newbranch nb, basebranch bb;
        `
        : `
          WITH basebranch AS (
            SELECT rb.branchid, b.projectid
            FROM recipe_branches rb
            JOIN branches b ON rb.branchid = b.id
            WHERE rb.recipeid = %s
            LIMIT 1
          ),
          newrecipe AS (
            INSERT INTO recipes (name, description, instructions, userid, additionalinfo, imageurls, status, notes)
            VALUES (%s, %s, %s, %s, %s, %s, %s, %s)
            RETURNING id
          ),
          newbranch AS (
            INSERT INTO branches (name, description, baserecipeid, headrecipeid, ownerid, projectid)
            SELECT %s, %s, %s, nr.id, %s, bb.projectid
            FROM newrecipe nr, basebranch bb
            RETURNING id, headrecipeid
          ),
          newbranchlink AS (
            INSERT INTO recipe_branches (recipeid, branchid, position, created_at)
            SELECT nr.id, nb.id,
                   COALESCE((SELECT MAX(position) + 1 FROM recipe_branches WHERE branchid = nb.id), 1),
                   NOW()
            FROM newrecipe nr, newbranch nb
            RETURNING recipeid
          ),
          newparent AS (
            INSERT INTO recipe_parents (recipeid, parentrecipeid)
            SELECT nr.id, %s
            FROM newrecipe nr
            RETURNING recipeid
          )
          SELECT nr.id AS recipeid, nb.id AS branchid, bb.projectid
          FROM newrecipe nr, newbranch nb, basebranch bb;
        `,
      values: ingredients.length > 0
        ? [
            recipeId, // for basebranch
            name,
            description,
            instructions,
            session.user.id,
            additionalinfo,
            imageurls,
            status,
            notes,
            branchName,
            branchDescription,
            recipeId, // baserecipeid
            session.user.id,
            recipeId // parentrecipeid
          ]
        : [
            recipeId, // for basebranch
            name,
            description,
            instructions,
            session.user.id,
            additionalinfo,
            imageurls,
            status,
            notes,
            branchName,
            branchDescription,
            recipeId, // baserecipeid
            session.user.id,
            recipeId // parentrecipeid
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
        return Response.json({ error: "Insert failed" }, { status: 400 });
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
