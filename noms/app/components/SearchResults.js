import { Box } from "@mui/material";
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
    const baseUrl = process.env.NOMS_URL || 'http://localhost:3000';

    const name = searchParams.name || ''
    const includedIngredients = searchParams.includedIngredients || '[]'
    const excludedIngredients = searchParams.excludedIngredients || '[]'
    const requiredIngredients = searchParams.requiredIngredients || '[]'

    const url = `${baseUrl}/api/search?name=${name}&includedIngredients=${includedIngredients}&excludedIngredients=${excludedIngredients}&requiredIngredients=${requiredIngredients}`
    const response = await fetch(url);

    if (!response.ok) {
      throw new Error(`HTTP error! Status: ${response.status}`);
    }

    const data = await response.json();
    return data.result || [];
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
