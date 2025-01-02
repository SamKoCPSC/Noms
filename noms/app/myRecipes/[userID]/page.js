import RecipeCard from "@/app/components/RecipeCard";
import { Container, Typography, Box } from "@mui/material";

export async function generateStaticParams() {
    const userIDs = ['1']
    return userIDs.map((id) => {
      return {userID: id}
    });
}

async function getUserRecipeData(id) {
    return fetch(
        `http://localhost:3000/api/myRecipes?id=${id}`
    ).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
}

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

export default async function({ params }) {
    const userRecipes = await getUserRecipeData(params.userID)

    const textStyle = {
        recipeTitleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return(
        <Container maxWidth='false' sx={{justifyItems: 'center', width: '70%'}}>
            <Box display={'flex'} flexDirection={'column'} flexWrap={'wrap'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.sectionTitleSize}}>My Recipes</Typography>
                {userRecipes.map((recipe, index) => {
                    return (
                        <RecipeCard
                            key={index}
                            id={recipe.id}
                            name={recipe.name}
                            description={recipe.description}
                            author={recipe.author}
                            date={formatTimestamp(recipe.datecreated)}
                            ingredients={recipe.ingredients}
                            imageURL={recipe.imageurls[0]}
                        />
                    )
                })}
            </Box>
        </Container>
    )
}