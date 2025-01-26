import RecipeCard from "@/app/components/RecipeCard";
import { Container, Typography, Box, Divider } from "@mui/material";
import AccessDenied from "@/app/components/AccessDenied";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/api/auth/[...nextauth]/route";

export async function generateStaticParams() {
    return fetch(
        `${process.env.NOMS_URL}/api/getAccount`
    ).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((user) => {
            return user.id.toString()
        })
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
}

async function getUserRecipeData(id) {
    return fetch(
        `${process.env.NOMS_URL}/api/myRecipes?id=${id}`
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
    const session = await getServerSession(authOptions)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    if(!session || 1 !== session.user.id) {
        return (
            <AccessDenied/>
        )
    }

    return(
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box display={'flex'} flexDirection={'column'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.titleSize, marginLeft: '150px'}}>My Recipes</Typography>
                <Divider width='90%'/>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.sectionTitleSize, marginLeft: '200px'}}>Personal Recipes</Typography>
                <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
                    {userRecipes.map((recipe, index) => { 
                        if(recipe.status === 'public') {
                            return (
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
                                    version={recipe.version}
                                />
                            )
                        }  
                    })}
                </Box>
                <Divider width='90%'/>
                <Typography sx={{alignSelf: 'start', fontSize: textStyle.sectionTitleSize, marginLeft: '200px'}}>Drafts</Typography>
                <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
                    {userRecipes.map((recipe, index) => { 
                        if(recipe.status === 'draft') {
                            return (
                                <RecipeCard
                                    key={index}
                                    id={recipe.id}
                                    name={recipe.name}
                                    description={recipe.description}
                                    author={recipe.author}
                                    date={formatTimestamp(recipe.datecreated)}
                                    ingredients={recipe.ingredients}
                                    imageURL={recipe.imageurls &&  recipe.imageurls[0]}
                                    status={recipe.status}
                                    baseid={recipe.baseid}
                                    version={recipe.version}
                                />
                            )
                        }  
                    })}
                </Box>
            </Box>
        </Container>
    )
}