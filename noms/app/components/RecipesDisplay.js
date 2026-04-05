import { Box } from "@mui/material";
import RecipeCard from "./RecipeCard";
import formatTimestamp from "../function/formatTimestamp";

async function fetchRecipes() {
  try {
    const baseUrl = process.env.NOMS_URL || 'http://localhost:3000';
    const response = await fetch(`${baseUrl}/api/getRecipes`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify({
        numOfResults: 24
      }),
      // Disable caching for dynamic content or set appropriate revalidation
      // next: { revalidate: 0 } // Set to 0 for no caching
    });

    if (!response.ok) {
      throw new Error('Failed to fetch recipes');
    }

    const data = await response.json();
    return data.result || [];
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
