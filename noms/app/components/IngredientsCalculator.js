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
    const [showPercentages, setShowPercentages] = useState(false);
    const [showBakersPercentages, setShowBakersPercentages] = useState(false);
    const [baseIngredientId, setBaseIngredientId] = useState('');

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

    const calculatePercentages = () => {
        const totalWeightInGrams = ingredients
            .filter(ing => unitToGrams[ing.unit.toLowerCase()])
            .reduce((sum, ing) => sum + ing.quantity * unitToGrams[ing.unit.toLowerCase()], 0);
        
        return ingredients.map(ingredient => {
            if (unitToGrams[ingredient.unit.toLowerCase()]) {
                const weightInGrams = ingredient.quantity * unitToGrams[ingredient.unit.toLowerCase()];
                const percentage = totalWeightInGrams > 0 ? (weightInGrams / totalWeightInGrams) * 100 : 0;
                return { ...ingredient, percentage: percentage.toFixed(1) };
            }
            return { ...ingredient, percentage: null };
        });
    };

    const calculateBakersPercentages = () => {
        if (!baseIngredientId) {
            return ingredients.map(ingredient => ({ ...ingredient, bakersPercentage: null }));
        }

        // Find the selected base ingredient
        const baseIngredient = ingredients.find(ingredient => 
            ingredient.id === baseIngredientId && unitToGrams[ingredient.unit.toLowerCase()]
        );

        if (!baseIngredient) {
            return ingredients.map(ingredient => ({ ...ingredient, bakersPercentage: null }));
        }

        // Calculate base ingredient weight in grams
        const baseWeightInGrams = baseIngredient.quantity * unitToGrams[baseIngredient.unit.toLowerCase()];

        if (baseWeightInGrams === 0) {
            return ingredients.map(ingredient => ({ ...ingredient, bakersPercentage: null }));
        }

        return ingredients.map(ingredient => {
            if (unitToGrams[ingredient.unit.toLowerCase()]) {
                const weightInGrams = ingredient.quantity * unitToGrams[ingredient.unit.toLowerCase()];
                const bakersPercentage = (weightInGrams / baseWeightInGrams) * 100;
                return { ...ingredient, bakersPercentage: bakersPercentage.toFixed(1) };
            }
            return { ...ingredient, bakersPercentage: null };
        });
    };

    const ingredientsWithPercentages = calculatePercentages();
    const ingredientsWithBakersPercentages = calculateBakersPercentages();
    
    // Combine both percentage calculations
    const ingredientsWithAllPercentages = ingredients.map((ingredient, index) => ({
        ...ingredient,
        percentage: ingredientsWithPercentages[index]?.percentage,
        bakersPercentage: ingredientsWithBakersPercentages[index]?.bakersPercentage
    }));

    // Get ingredients that can be used as base (gravimetric units only)
    const availableBaseIngredients = ingredients.filter(ingredient => 
        unitToGrams[ingredient.unit.toLowerCase()]
    );

    // Auto-set base ingredient to first available if not set and BP is being shown
    if (showBakersPercentages && !baseIngredientId && availableBaseIngredients.length > 0) {
        setBaseIngredientId(availableBaseIngredients[0].id);
    }

    // Check if baker's percentages are available (has base ingredient selected)
    const hasBakersPercentages = baseIngredientId && ingredientsWithBakersPercentages.some(ing => ing.bakersPercentage !== null);

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
                <Typography sx={{justifySelf: 'left', fontSize: textStyle.sectionTitleSize, marginBottom: 2}}>Ingredients</Typography>
                {ingredientsWithAllPercentages?.map((ingredient, index) => {
                    return (
                        <Box key={index} display="flex" alignItems="center" marginBottom="5px">
                            <Typography sx={{fontSize: textStyle.paragraphSize, marginRight: '10px'}}>
                                {Number(ingredient.quantity.toFixed(3))}{ingredient.unit} {ingredient.name}
                            </Typography>
                            {showPercentages && ingredient.percentage !== null && (
                                <Typography sx={{fontSize: textStyle.paragraphSize, color: 'gray', fontStyle: 'italic', marginRight: '10px'}}>
                                    ({ingredient.percentage}%)
                                </Typography>
                            )}
                            {showBakersPercentages && ingredient.bakersPercentage !== null && (
                                <Typography sx={{fontSize: textStyle.paragraphSize, color: theme.palette.info.main, fontStyle: 'italic'}}>
                                    [BP: {ingredient.bakersPercentage}%]
                                </Typography>
                            )}
                        </Box>
                    )
                })}
                {showBakersPercentages && availableBaseIngredients.length > 0 && (
                    <Box display="flex" alignItems="center" gap={2} marginBottom={1}>
                        <Typography variant="body2">Base ingredient for BP:</Typography>
                        <Select
                            value={baseIngredientId}
                            onChange={(e) => setBaseIngredientId(e.target.value)}
                            displayEmpty
                            size="small"
                            sx={{ minWidth: 150 }}
                        >
                            <MenuItem value="">Select Base</MenuItem>
                            {availableBaseIngredients.map((ingredient) => (
                                <MenuItem key={ingredient.id} value={ingredient.id}>
                                    {ingredient.name}
                                </MenuItem>
                            ))}
                        </Select>
                    </Box>
                )}
                <Box display="flex" alignItems="center" gap={1} flexWrap="wrap">
                    <Typography>
                        Total Weight: {ingredients
                            .filter(ing => unitToGrams[ing.unit.toLowerCase()])
                            .reduce((sum, ing) => sum + ing.quantity * unitToGrams[ing.unit.toLowerCase()], 0).toFixed(1)}g
                    </Typography>
                    <Button 
                        variant="contained" 
                        size="small"
                        onClick={() => setShowPercentages(!showPercentages)}
                        disabled={!isAllGravimetric}
                    >
                        Proportions
                    </Button>
                    <Button 
                        variant="contained" 
                        size="small"
                        onClick={() => setShowBakersPercentages(!showBakersPercentages)}
                        disabled={availableBaseIngredients.length === 0}
                    >
                        Ratios
                    </Button>
                </Box>
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