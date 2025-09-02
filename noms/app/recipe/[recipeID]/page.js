import Carousel from "@/app/components/Carousel";
import { Box, Button, Container, Divider, Typography } from "@mui/material"
import Link from "next/link";
import IngredientsCalculator from "@/app/components/IngredientsCalculator"
import BranchSelector from "@/app/components/BranchSelector";

export async function generateStaticParams() {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT id
                FROM recipes;
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((id) => {
            return {
                id: id.id
            }
        })
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

async function getRecipeData(id) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql:  `
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
                    -- Aggregate ingredients separately
                    (
                        SELECT json_agg(
                            json_build_object(
                                'id', i.id,
                                'name', i.name,
                                'quantity', ri.quantity,
                                'unit', ri.unit
                            )
                        )
                        FROM recipe_ingredients ri
                        JOIN ingredients i ON ri.ingredientid = i.id
                        WHERE ri.recipeid = r.id
                    ) AS ingredients,
                    -- Aggregate branches separately
                    (
                        SELECT json_agg(
                            json_build_object(
                                'branchid', rb.branchid,
                                'branchname', b.name,
                                'position', rb.position,
                                'created_at', rb.created_at
                            ) ORDER BY rb.created_at
                        )
                        FROM recipe_branches rb
                        JOIN branches b ON rb.branchid = b.id
                        WHERE rb.recipeid = r.id
                    ) AS branches
                FROM recipes r
                LEFT JOIN users u ON r.userid = u.id
                WHERE r.id = %s
                GROUP BY r.id, u.name;
            `,
            values: [id]
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result[0]
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
}

export default async function Recipe({ params }) {
    const recipeData = await getRecipeData(params.recipeID)

    const textStyle = {
        recipeTitleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return (
        <Container
            sx={{
            marginTop: '65px',
            width: '100%',
            justifyItems: 'center',
            }}
        >
            <Box display={'flex'} flexDirection={{width800: 'row', sm: 'column', xs: 'column'}} 
                sx={{
                    width: '100%',
                    justifyContent: 'space-between',
                    backgroundColor: 'white', 
                    padding: '20px',
                    margin: '30px',
                    borderRadius: '30px',
                    borderColor: 'rgb(230, 228, 215)',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
                }}>
                <Box display={'flex'} flexDirection={'column'} sx={{ gap: '20px'}}>
                    <Typography>Iteration: {recipeData.branches[0].position}</Typography>
                    <Typography>Notes: {recipeData.notes ? recipeData.notes : 'None'}</Typography>
                </Box>
                <Box display={'flex'} flexDirection={{width550: 'row', xs: 'column'}} sx={{justifyContent: 'center'}}>
                    <Box display={'flex'} sx={{justifyContent: 'center'}}>
                        {/* <Link href={`/tree/${recipeData.baseid}`}>
                            <Button variant="contained" color='secondary'>
                                View Tree
                            </Button>
                        </Link> */}
                        {/* <Link key={recipeData.branches[0].branchid} href={`/branch/${recipeData.branches[0].branchid}`} passHref>
                            <Button variant="contained" color="secondary">
                                View Branch
                            </Button>
                        </Link> */}
                        <BranchSelector branches={recipeData.branches}/>
                    </Box>
                    <Box display={'flex'} sx={{justifyContent: 'center'}}>
                        <Link href={`/createRecipe?name=${recipeData.name}&description=${recipeData.description}&ingredients=${JSON.stringify(recipeData.ingredients)}&instructions=${JSON.stringify(recipeData.instructions)}&additionalInfo=${JSON.stringify(recipeData.additionalinfo)}&imageURLs=${JSON.stringify(recipeData.imageurls)}`}>
                            <Button variant="contained">
                                New Branch
                            </Button>
                        </Link>
                        <Link href={`/createRecipe?name=${recipeData.name}&description=${recipeData.description}&ingredients=${JSON.stringify(recipeData.ingredients)}&instructions=${JSON.stringify(recipeData.instructions)}&additionalInfo=${JSON.stringify(recipeData.additionalinfo)}&imageURLs=${JSON.stringify(recipeData.imageurls)}&branchid=${recipeData.branches[0].branchid}`}>
                            <Button variant="contained">
                                New Version
                            </Button>
                        </Link>
                    </Box>
                </Box>
            </Box>
            <Divider sx={{marginTop: '30px', width: '100%'}}/>
            <Typography sx={{justifySelf: 'center', fontSize: textStyle.recipeTitleSize}}>{recipeData.name}</Typography>
            <Typography sx={{justifySelf: 'center', fontSize: textStyle.paragraphSize, textAlign: 'center'}}>{recipeData.description}</Typography>
            <Divider sx={{marginY: '30px', width: '100%'}}/>
            <Carousel 
            slides={recipeData.imageurls.map((imageurl, index) => {
                return (
                    <Box 
                        key={index}
                        component={'img'}
                        alt="image"
                        src={imageurl}
                        height='90%'
                    /> 
                )
            })}
            slidesPerView={1}
            height={{width800: '500px', width550: '400px', xs: '300px'}}
            />
            <IngredientsCalculator ingredientsProps={recipeData.ingredients}/>
            {/* <Box display={'flex'} flexDirection={'row'}
            sx={{
                backgroundColor: 'white', 
                width: '100%', 
                padding: '20px',
                marginTop: '30px',
                justifyContent: 'space-between',
                borderRadius: '30px',
                borderColor: 'rgb(230, 228, 215)',
                borderStyle: 'solid',
                borderWidth: 2,
                boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
            }}>
                <Box>
                    <Typography sx={{justifySelf: 'left', fontSize: textStyle.sectionTitleSize}}>Ingredients</Typography>
                    {recipeData.ingredients?.map((ingredient, index) => {
                        return <Typography key={index} sx={{justifySelf: 'left', fontSize: textStyle.paragraphSize, marginBottom: '5px'}}>{ingredient.quantity}{ingredient.unit} {ingredient.name}</Typography>
                    })}
                </Box>
                <Box display={'flex'} flexDirection={'row'}>
                    <Button variant="contained" sx={{height: '50px'}}>Scale</Button>
                </Box>
            </Box> */}
            <Box
            sx={{
                backgroundColor: 'white', 
                width: '100%', 
                padding: '20px',
                marginTop: '30px',
                borderRadius: '30px',
                borderColor: 'rgb(230, 228, 215)',
                borderStyle: 'solid',
                borderWidth: 2,
                boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
            }}>
            <Typography sx={{justifySelf: 'left', fontSize: textStyle.sectionTitleSize}}>Instructions</Typography>
            {recipeData.instructions?.map((instruction, index) => {
                return (
                    <Box key={index} display={'flex'} flexDirection={'column'} sx={{justifySelf: 'left'}}>
                        <Typography sx={{justifySelf: 'left', fontSize: textStyle.listItemSize}}>
                            {instruction.title}
                        </Typography>
                        <Typography sx={{justifySelf: 'left', fontSize: textStyle.paragraphSize, marginBottom: '15px'}}>
                            {instruction.instruction}
                        </Typography>
                    </Box>
                )
            })} 
            </Box>
            <Divider sx={{marginY: '30px', width: '100%'}}/>
            {recipeData.additionalinfo?.map((info, index) => {
                return (
                    <Box key={index} display={'flex'} flexDirection={'column'} sx={{justifySelf: 'left'}}>
                        <Typography sx={{justifySelf: 'left', fontSize: textStyle.listItemSize}}>
                            {info.title}
                        </Typography>
                        <Typography sx={{justifySelf: 'left', fontSize: textStyle.paragraphSize, marginBottom: '15px'}}>
                            {info.info}
                        </Typography>
                    </Box>
                )
            })} 
            <Divider sx={{marginY: '30px', width: '100%'}}/>
        </Container>
    )
}