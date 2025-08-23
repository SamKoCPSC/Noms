'use client'
import { Box, Stack, Typography, Button, TextField, Select, MenuItem } from "@mui/material";
import theme from "../theme";
import { useState } from "react";

export default function IngredientsCalculator({ ingredientsProps }) {
    const [originalIngredients] = useState(ingredientsProps)
    const [ingredients, setIngredients] = useState(ingredientsProps)
    const [scaleMode, setScaleMode] = useState(false)
    const [scalingFactor, setScalingFactor] = useState('1')
    const [selectedIngredient, setSelectedIngredient] = useState('');
    const [targetQuantity, setTargetQuantity] = useState('');
    const [targetTotalWeight, setTargetTotalWeight] = useState('');

    const handleMultiplierScale = () => {
        setIngredients(ingredients.map((ingredient) => {
            return {
                id: ingredient.id,
                name: ingredient.name,
                quantity: ingredient.quantity * parseFloat(scalingFactor) || ingredient.quantity,
                unit: ingredient.unit
            }
        }))
        setScaleMode(false)
    }

    const handleResetQuantities = () => {
        setIngredients(originalIngredients);
        setScalingFactor('1');
        setSelectedIngredient('');
        setTargetQuantity('');
        setScaleMode(false);
    }

    const handleIngredientScale = () => {
        const ingredient = originalIngredients.find(i => i.id === selectedIngredient);
        if (!ingredient) return;
    
        const newQty = parseFloat(targetQuantity);
        if (isNaN(newQty) || newQty <= 0) return;
    
        const factor = newQty / ingredient.quantity;
    
        setIngredients(originalIngredients.map((ing) => ({
          ...ing,
          quantity: ing.quantity * factor
        })));
        setScaleMode(false);
    };

    const handleWeightScale = () => {
        const newTotal = parseFloat(targetTotalWeight);
        if (isNaN(newTotal) || newTotal <= 0) return;
      
        const currentTotal = ingredients.reduce((sum, ing) => sum + ing.quantity, 0);
        const factor = newTotal / currentTotal;
      
        setIngredients(ingredients.map(ing => ({
          ...ing,
          quantity: ing.quantity * factor
        })));
      
        setTargetTotalWeight('');
        setScaleMode(false);
      };

    const textStyle = {
        recipeTitleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    const gravimetricUnits = ['g', 'kg', 'mg', 'oz', 'lb']
    const isAllGravimetric = ingredients.every(ingredient => 
        gravimetricUnits.includes(ingredient.unit.toLowerCase())
    )

    const unitToGrams = {
        g: 1,
        kg: 1000,
        mg: 0.001,
        oz: 28.3495,
        lb: 453.592,
      };

    return (
        <Box display={'flex'} flexDirection={{width470: 'row', xs: 'column'}}
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
                    return <Typography key={index} sx={{justifySelf: 'left', fontSize: textStyle.paragraphSize, marginBottom: '5px'}}>{Number(ingredient.quantity.toFixed(3))}{ingredient.unit} {ingredient.name}</Typography>
                })}
                <Typography>
                    Total Weight: {ingredients
                        .filter(ing => unitToGrams[ing.unit.toLowerCase()])
                        .reduce((sum, ing) => sum + ing.quantity * unitToGrams[ing.unit.toLowerCase()], 0)}g
                </Typography>
            </Box>
            {scaleMode ? 
                <Box display={'flex'} flexDirection={'column'} alignItems={'end'}>
                    
                    <Box display={'flex'} flexDirection={'row'}>
                        <TextField
                            label={'Multiplier'}
                            sx={{width: '120px'}}
                            onChange={(event) => setScalingFactor(event.target.value)}
                        />
                        <Button variant="contained" onClick={() => handleMultiplierScale()}>Scale by Multiplier</Button>
                        
                    </Box>
                    <Box display="flex" flexDirection={'row'}>
                        <TextField
                            label='Total Weight'
                            value={targetTotalWeight}
                            onChange={(e) => setTargetTotalWeight(e.target.value)}
                            sx={{ width: '120px' }}
                            disabled={!isAllGravimetric}
                        />
                        <Button variant="contained" onClick={handleWeightScale} disabled={!isAllGravimetric}>
                            Scale by Total Weight
                        </Button>
                    </Box>
                     {/* New: Scale by ingredient */}
                     <Box display="flex" gap={1}>
                        <Select
                        value={selectedIngredient}
                        onChange={(e) => setSelectedIngredient(e.target.value)}
                        displayEmpty
                        sx={{ width: 180 }}
                        >
                        <MenuItem value="">Select Ingredient</MenuItem>
                        {originalIngredients.map((ingredient) => (
                            <MenuItem key={ingredient.id} value={ingredient.id}>
                            {ingredient.name}
                            </MenuItem>
                        ))}
                        </Select>
                        <TextField
                        label="Target Quantity"
                        value={targetQuantity}
                        onChange={(e) => setTargetQuantity(e.target.value)}
                        sx={{ width: '120px' }}
                        />
                        <Button variant="contained" onClick={handleIngredientScale}>Scale by Ingredient</Button>
                    </Box>
                        <Button variant="contained" color="warning" sx={{height: '50px'}} onClick={() => setScaleMode(false)}>Cancel</Button>
                </Box>
            : 
                <Box display={'flex'} flexDirection={'row'}>
                    <Button variant="contained" sx={{height: '50px'}} onClick={() => setScaleMode(true)}>Scale</Button>
                    <Button variant="contained" color="warning" sx={{height: '50px'}} onClick={handleResetQuantities}>Reset</Button>
                </Box>
            }
        </Box>
    )
}