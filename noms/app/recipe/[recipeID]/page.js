'use client'

import { Box, Container, Typography } from "@mui/material"
import Navbar from '../../components/Navbar'

export default function Recipe() {
    const recipeData = {
        title: 'Croissant',
        description: 'This classic croissant recipe produces golden, buttery, and flaky pastries that melt in your mouth. Perfectly laminated layers of dough are crafted with patience and care, filled with rich butter, and baked to perfection. Enjoy these delectable croissants fresh from the oven as a breakfast treat or a delightful snack.',
        date: "December 6, 2024",
        ingredients: [
            {name: 'Flour', unit: 'g', amount: '400'},
            {name: 'Water', unit: 'g', amount: '160'},
            {name: 'Salt', unit: 'g', amount: '8'},
            {name: 'Yeast', unit: 'g', amount: '12'},
            {name: 'Sugar', unit: 'g', amount: '45'}
        ],
        instructions: [
            {step: 'Prepate the Dough', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
            {step: 'Prepare the Butter', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
            {step: 'Laminate the Dough', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
            {step: 'Shape the Croissants', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
        ]
        }

    return (
        <Container
            sx={{
            marginTop: '65px',
            width: '70%',
            height: '100vh',
            justifyItems: 'center',
            backgroundColor: '#d1d1d1'
            }}
        >
            <Navbar></Navbar>
            <Typography sx={{justifySelf: 'center', fontSize: '50px'}}>{recipeData.title}</Typography>
            <Typography sx={{justifySelf: 'center', fontSize: '20px'}}>{recipeData.description}</Typography>
            <Box 
                component={'img'}
                sx={{
                    height: 400,
                    width: 725,
                }}
                alt="Croissant"
                src="/croissant1.jpg"
            >
            </Box>
            <Typography sx={{justifySelf: 'left', fontSize: '35px'}}>Ingredients</Typography>
            {recipeData.ingredients.map((ingredient, index) => {
                return <Typography sx={{justifySelf: 'left', fontSize: '20px'}}>{ingredient.amount}{ingredient.unit} {ingredient.name}</Typography>
            })}
            <Typography sx={{justifySelf: 'left', fontSize: '35px'}}>Instructions</Typography>
            {recipeData.instructions.map((instruction) => {
                return (
                    <Box sx={{justifySelf: 'left'}}>
                        <Typography sx={{justifySelf: 'left', fontSize: '25px'}}>
                            {instruction.step}
                        </Typography>
                        {instruction.details.map((subInstruction) => {
                            return <Typography sx={{justifySelf: 'left', fontSize: '20px'}}>-{subInstruction}</Typography>
                        })}
                    </Box>
                )
            })} 
        </Container>
    )
}