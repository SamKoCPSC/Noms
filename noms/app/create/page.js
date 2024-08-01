'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import { useFormik } from "formik";
import Navbar from '../components/Navbar'
import {Box, Container, Divider, Stack, TextField, Typography, Button, MenuItem} from "@mui/material";

const units = ['g', 'mL']

export default function Create() {
  const [addIngredientMode, setIngredientMode] = React.useState(false)
  const [ingredients, setIngredients] = React.useState([{quantity: 2, unit: 'g', name: 'Salt'}])

  const ingredientFormik = useFormik({
    initialValues: {
        quantity: '',
        unit: '', 
        name: ''
    },
    onSubmit: (values) => {
        console.log('hi')
    }
  })

  const formik = useFormik({
    initialValues: {
        title: ''
    }
  })

  const handleIngredientMode = () => {
    setIngredientMode(!addIngredientMode)
  }

  return (
    <Container>
      <Box left='0%' width={'100%'}>
        <Navbar></Navbar>
      </Box>
      <main className={styles.main}>
        
        <Box width='800px' display={"flex"} flexDirection={'column'} sx={{gap: '10px'}}> 
          <Typography fontSize={'50px'}>
            Create A Nom
          </Typography>
          <Divider sx={{margin: '10px'}}></Divider>
          <Stack direction='row'>
            <Typography fontSize="25px">Name:</Typography>
            <TextField variant="outlined" fullWidth sx={{margin: '5px'}}></TextField>
          </Stack>
          <Box>
            <Typography fontSize="25px">Ingredients:</Typography>
            {ingredients.map((ingredient) => (
                <Typography>{ingredient.quantity + ingredient.unit + ' ' + ingredient.name}</Typography>
            ))}
            {addIngredientMode ? 
                <Box display="flex">
                    <Typography>Ingredient</Typography>
                    <TextField 
                        sx={{margin: '5px'}} 
                        value={ingredientFormik.values.name}
                        id="name"
                        onChange={ingredientFormik.handleChange}>
                    </TextField>
                    <Typography>Quantity</Typography>
                    <TextField 
                        sx={{margin: '5px'}} 
                        value={ingredientFormik.values.quantity}
                        id="quantity"
                        onChange={ingredientFormik.handleChange}>
                    </TextField>
                    <Typography>Unit</Typography>
                    <TextField 
                        select 
                        sx={{margin: '5px'}} 
                        value={ingredientFormik.values.unit}
                        id="unit"
                        onChange={ingredientFormik.handleChange('unit')}>
                            {units.map((unit) => (
                                <MenuItem key={unit} value={unit}>{unit}</MenuItem>
                            ))}
                    </TextField>
                </Box> : 
                <></>
            }
            {addIngredientMode ? 
                <Box>
                    <Button 
                        onClick={() => {
                            handleIngredientMode() 
                            ingredientFormik.handleSubmit() 
                            ingredientFormik.resetForm()}}
                    >
                        Confirm
                    </Button>
                    <Button 
                        onClick={() => {
                            handleIngredientMode()
                            ingredientFormik.resetForm()}}
                    >
                        Cancel
                    </Button>
                </Box> : 
                <Button onClick={() => handleIngredientMode()}>+ Add Ingredient</Button>
            }
          </Box>
          <Box>
            <Typography fontSize="25px">Instructions:</Typography>
            <TextField variant="outlined" fullWidth></TextField>
          </Box>
          <Divider sx={{margin: '10px'}}></Divider>
          <Box>
            <Typography fontSize='25px'>Notes:</Typography>
            <TextField variant="outlined" fullWidth></TextField>
          </Box>
          <Button variant="filled" sx={{alignSelf: 'end', backgroundColor: 'lightblue'}}>Create</Button>
        </Box>
      </main>
    </Container>
    
  );
}
