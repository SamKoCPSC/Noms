import Carousel from "@/app/components/Carousel";
import { Box, Button, Container, Divider, Typography } from "@mui/material"
import { revalidatePath } from "next/cache";
import Link from "next/link";

export async function generateStaticParams() {
    const recipeIDs = ['1']
    return recipeIDs.map((id) => {
      return {recipeID: id}
    });
}

async function getRecipeData(id) {
    return fetch(
        `http://localhost:3000/api/getRecipe?id=${id}`
    ).then((response) => {
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
            width: '70%',
            justifyItems: 'center',
            }}
        >
            <Box display={'flex'} flexDirection={'row'} sx={{width: '100%', justifyContent: 'end'}}>
                <Button variant="contained">View All Versions</Button>
                <Link href={`/create?name=${recipeData.name}&description=${recipeData.description}&ingredients=${JSON.stringify(recipeData.ingredients)}&instructions=${JSON.stringify(recipeData.instructions)}&additionalInfo=${JSON.stringify(recipeData.additionalinfo)}&imageURLs=${JSON.stringify(recipeData.imageurls)}&baseid=${recipeData.baseid}&branchbase=${recipeData.recipeid}`}>
                    <Button variant="contained">
                        New Branch
                    </Button>
                </Link>
                <Link href={`/create?name=${recipeData.name}&description=${recipeData.description}&ingredients=${JSON.stringify(recipeData.ingredients)}&instructions=${JSON.stringify(recipeData.instructions)}&additionalInfo=${JSON.stringify(recipeData.additionalinfo)}&imageURLs=${JSON.stringify(recipeData.imageurls)}&baseid=${recipeData.baseid}${recipeData.branchbase ? `&branchbase=${recipeData.branchbase}` : ''}&branchid=${recipeData.branchid}`}>
                    <Button variant="contained">
                        New Version
                    </Button>
                </Link>
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
            height='500px'
            />
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
            <Typography sx={{justifySelf: 'left', fontSize: textStyle.sectionTitleSize}}>Ingredients</Typography>
            {recipeData.ingredients?.map((ingredient, index) => {
                return <Typography key={index} sx={{justifySelf: 'left', fontSize: textStyle.paragraphSize, marginBottom: '5px'}}>{ingredient.quantity}{ingredient.unit} {ingredient.name}</Typography>
            })}
            </Box>
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