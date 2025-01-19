import { Typography, Container, Divider, Box, Avatar } from "@mui/material";
import RecipeCard from "@/app/components/RecipeCard";

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

export async function generateStaticParams() {
    const recipeIDs = ['1']
    return recipeIDs.map((id) => {
      return {recipeID: id}
    });
}

async function getBranchRecipes(branchbase, branchid) {
    return fetch(
        `${process.env.NOMS_URL}/api/getRecipeBranch?branchbase=${branchbase}&branchid=${branchid}`
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

export default async function Recipe({ params }) {
    const branchRecipes = await getBranchRecipes(params.branchbase, params.branchid)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return (
        // <Typography>{JSON.stringify(branchRecipes)}</Typography>
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box display={'flex'} flexDirection={'column'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.titleSize, marginLeft: '150px'}}>Branch</Typography>
                <Divider width='90%'/>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.sectionTitleSize, marginLeft: '200px'}}>Personal Recipes</Typography>
                <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
                    {branchRecipes.map((recipe, index) => { 
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
                                    branchid = {recipe.branchid}
                                    branchbase = {recipe.branchbase}
                                />
                            )
                        }  
                    })}
                </Box>
            </Box>
        </Container>
    )
}