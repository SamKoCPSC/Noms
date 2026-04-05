import { Box } from "@mui/material";
import axios from "axios";
import RecipeCard from "./RecipeCard";

function formatTimestamp(timestamp) {
  const isoTimestamp = timestamp.replace(" ", "T");
  const date = new Date(isoTimestamp);
  if (isNaN(date.getTime())) {
      throw new Error("Invalid PostgreSQL timestamp format.");
  }
  const options = {
      year: "numeric",
      month: "long",
      day: "numeric",
  };
  return date.toLocaleDateString(undefined, options);
}

async function fetchSearchResults(searchParams) {
  try {
    const name = `%${searchParams.name || ''}%`

    const parseJsonArray = (value) => {
      if (!value) return []
      try {
        const parsed = JSON.parse(value)
        return Array.isArray(parsed) ? parsed : []
      } catch (error) {
        return []
      }
    }

    const includedIngredientsParsed = parseJsonArray(searchParams.includedIngredients)
    const includedIngredients = includedIngredientsParsed.length > 0 ? includedIngredientsParsed : ['%']
    const excludedIngredients = parseJsonArray(searchParams.excludedIngredients)
    const requiredIngredients = parseJsonArray(searchParams.requiredIngredients)

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

    const response = await axios.post(
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
    );

    return response.data.result || [];
  } catch (error) {
    console.error('Error fetching search results:', error);
    return [];
  }
}

export default async function SearchResults({ searchParams }) {
  const recipes = await fetchSearchResults(searchParams);

  return (
    <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
      {recipes.map((recipe, index) => { 
        if(recipe.status === 'public') {
          return (
            <RecipeCard
              key={index}
              id={recipe.recipeid}
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
              version={recipe.version}
              notes={recipe.notes}
              branchid={recipe.branchid}
              branchbase={recipe.branchbase}
            />
          )
        }  
      })}
    </Box>
  );
}
