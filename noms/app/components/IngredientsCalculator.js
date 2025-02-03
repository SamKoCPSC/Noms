'use client'
import { Box, Stack, Typography, Button, TextField } from "@mui/material";
import theme from "../theme";
import { useState } from "react";

export default function IngredientsCalculator({ ingredientsProps }) {
    const [ingredients, setIngredients] = useState(ingredientsProps)
    const [scaleMode, setScaleMode] = useState(false)
    const [scalingFactor, setScalingFactor] = useState('1')

    const handleScale = () => {
        setIngredients(ingredients.map((ingredient) => {
            return {
                id: ingredient.id,
                name: ingredient.name,
                quantity: ingredient.quantity * scalingFactor,
                unit: ingredient.unit
            }
        }))
        setScaleMode(false)
    }

    const textStyle = {
        recipeTitleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return (
        <Box display={'flex'} flexDirection={'row'}
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
                {ingredients?.map((ingredient, index) => {
                    return <Typography key={index} sx={{justifySelf: 'left', fontSize: textStyle.paragraphSize, marginBottom: '5px'}}>{ingredient.quantity}{ingredient.unit} {ingredient.name}</Typography>
                })}
            </Box>
            {scaleMode ? 
                <Box display={'flex'} flexDirection={'column'}>
                    <TextField
                        onChange={(event) => setScalingFactor(event.target.value)}
                    />
                    <Box display={'flex'} flexDirection={'row'}>
                        <Button variant="contained" sx={{height: '50px'}} onClick={() => handleScale()}>Confirm</Button>
                        <Button variant="contained" color="warning" sx={{height: '50px'}} onClick={() => setScaleMode(false)}>Cancel</Button>
                    </Box>
                </Box>
            : 
                <Box display={'flex'} flexDirection={'row'}>
                    <Button variant="contained" sx={{height: '50px'}} onClick={() => setScaleMode(true)}>Scale</Button>
                </Box>
            }
        </Box>
    )
}