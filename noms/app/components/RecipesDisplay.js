import { Box } from "@mui/material";
import axios from "axios";
import RecipeCard from "./RecipeCard";
import formatTimestamp from "../function/formatTimestamp";

async function fetchRecipes() {
  try {
    const response = await axios.post(
      process.env.LAMBDA_API_URL,
      {
        sql: `
          WITH distinct_branches AS (
            SELECT DISTINCT ON (b.baserecipeid) b.headrecipeid
            FROM branches b
            JOIN recipes r ON r.id = b.headrecipeid
            WHERE r.status = 'public'
            ORDER BY b.baserecipeid, RANDOM()
            LIMIT %s
          )
          SELECT
            r.*,
            u.name AS author,
            json_agg(
              json_build_object(
                'id', i.id,
                'name', i.name,
                'quantity', ri.quantity,
                'unit', ri.unit
              )
            ) AS ingredients
          FROM distinct_branches db
          JOIN recipes r ON r.id = db.headrecipeid
          LEFT JOIN recipe_ingredients ri ON r.id = ri.recipeid
          LEFT JOIN ingredients i ON ri.ingredientid = i.id
          LEFT JOIN users u ON r.userid = u.id
          GROUP BY r.id, u.name;
        `,
        values: [24]
      },
      {
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': process.env.LAMBDA_API_KEY,
        }
      }
    );

    return response.data.result || [];
  } catch (error) {
    console.error('Error fetching recipes:', error);
    return [];
  }
}

export default async function RecipesDisplay() {
  const randomRecipes = await fetchRecipes();

  return (
    <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
      {randomRecipes.map((recipe, index) => (
        <RecipeCard 
          key={index}
          id={recipe.id}
          name={recipe.name}
          description={recipe.description}
          author={recipe.author}
          date={formatTimestamp(recipe.datecreated)}
          ingredients={recipe.ingredients}
          instructions={recipe.instructions}
          additionalInfo={recipe.additionalinfo}
          imageURLs={recipe.imageurls}
          status={recipe.status}
          baseid={recipe.baseid}
          notes={recipe.notes}
          branchid={recipe.branchid}
          branchbase={recipe.branchbase}
        />
      ))}
    </Box>
  );
}
