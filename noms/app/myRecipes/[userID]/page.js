import RecipeCard from "@/app/components/RecipeCard";
import { Container, Typography, Box, Divider } from "@mui/material";
import AccessDenied from "@/app/components/AccessDenied";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/api/auth/[...nextauth]/route";

export async function generateStaticParams() {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT *
                FROM users
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((user) => ({
            userID: user.id.toString()
        }))
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
}

async function getUserRecipeData(id) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT 
                    r.id AS id,
                    r.name AS name,
                    r.description,
                    r.instructions,
                    r.datecreated,
                    r.additionalinfo,
                    r.imageurls,
                    r.status,
                    u.name AS author,
                    u.email
                FROM 
                    recipes r
                JOIN 
                    users u ON r.userid = u.id
                WHERE 
                    u.id = %s;
            `,
            values: [id]
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result
    })
    .catch((error) => {
        console.error(error)
        return []
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

    if(!session || params.userID !== session.user.id.toString()) {
        return (
            <AccessDenied/>
        )
    }

    return(
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box display={'flex'} flexDirection={'column'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Typography sx={{alignSelf: {width800: 'start', xs: 'center'}, fontSize: textStyle.titleSize, marginLeft: {width800: '150px', xs: '0px'}}}>My Recipes</Typography>
                <Divider width='90%'/>
                <Typography sx={{alignSelf: {width800: 'start', xs: 'center'}, fontSize: textStyle.sectionTitleSize, marginLeft: {width800: '200px', xs: '0px'}}}>Personal Recipes</Typography>
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
                {/* <Divider width='90%'/>
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
                </Box> */}
            </Box>
        </Container>
    )
}