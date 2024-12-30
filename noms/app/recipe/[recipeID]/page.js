import axios from "axios";

import { Box, Container, Typography } from "@mui/material"
import Navbar from '../../components/Navbar'
import { ReceiptOutlined } from "@mui/icons-material";

export async function generateStaticParams() {
    const recipeIDs = ['1']
  
    // Return an array of params objects
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

    return (
        <Container
            sx={{
            marginTop: '65px',
            width: '70%',
            justifyItems: 'center',
            backgroundColor: '#d1d1d1'
            }}
        >
            <Navbar></Navbar>
            <Typography sx={{justifySelf: 'center', fontSize: '50px'}}>{recipeData.name}</Typography>
            <Typography sx={{justifySelf: 'center', fontSize: '20px'}}>{recipeData.description}</Typography>
            <Box 
                component={'img'}
                sx={{
                    height: 400,
                    width: 725,
                }}
                alt="Croissant"
                src={recipeData.imageurls[0]}
            >
            </Box>
            <Typography sx={{justifySelf: 'left', fontSize: '35px'}}>Ingredients</Typography>
            {recipeData.ingredients?.map((ingredient) => {
                return <Typography sx={{justifySelf: 'left', fontSize: '20px'}}>{ingredient.quantity}{ingredient.unit} {ingredient.name}</Typography>
            })}
            <Typography sx={{justifySelf: 'left', fontSize: '35px'}}>Instructions</Typography>
            {recipeData.instructions?.map((instruction) => {
                return (
                    <Box sx={{justifySelf: 'left'}}>
                        <Typography sx={{justifySelf: 'left', fontSize: '25px'}}>
                            {instruction.title}
                        </Typography>
                        <Typography sx={{justifySelf: 'left', fontSize: '16px'}}>
                            {instruction.instruction}
                        </Typography>
                    </Box>
                )
            })} 
        </Container>
    )
}