'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import { useFormik } from "formik";
import Navbar from '../components/Navbar'
import EditIcon from '@mui/icons-material/Edit';
import {Box, Container, Divider, Stack, TextField, Typography, Button, MenuItem, Checkbox} from "@mui/material";

const units = ['g', 'mL']

export default function Create() {
  const [addIngredientMode, setAddIngredientMode] = React.useState(false)
  const [editIngredientMode, setEditIngredientMode] = React.useState(false)
  const [ingredients, setIngredients] = React.useState([])
  const [selectedIngredients, setSelectedIngredients] = React.useState([]);

  const ingredientFormik = useFormik({
    initialValues: {
        quantity: '',
        unit: '', 
        name: ''
    },
    onSubmit: (values, actions) => {
        console.log(values)
        setIngredients([...ingredients, {quantity: values.quantity, unit: values.unit, name: values.name}])
        ingredientFormik.resetForm()
    }
  })

  const formik = useFormik({
    initialValues: {
        title: ''
    }
  })

  const handleAddIngredientMode = () => {
    setAddIngredientMode(!addIngredientMode)
  }

  const handleEditIngredientMode = () => {
    setEditIngredientMode(!editIngredientMode)
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
            {ingredients.map((ingredient, index) => (
                editIngredientMode ? 
                <Box>
                  <Typography key={index}>
                    <Checkbox sx={{padding: '0px'}}></Checkbox>
                    {ingredient.quantity + ingredient.unit + ' ' + ingredient.name}
                  </Typography>
                </Box> : 
                <Typography key={index}>{ingredient.quantity + ingredient.unit + ' ' + ingredient.name}</Typography>
            ))}
            {addIngredientMode ? 
                <Box display="flex">
                    <Typography>Ingredient</Typography>
                    <TextField 
                        sx={{margin: '5px'}} 
                        value={ingredientFormik.values.name}
                        id="name"
                        name="name"
                        onChange={ingredientFormik.handleChange}>
                    </TextField>
                    <Typography>Quantity</Typography>
                    <TextField 
                        sx={{margin: '5px'}} 
                        value={ingredientFormik.values.quantity}
                        id="quantity"
                        name="quantity"
                        onChange={ingredientFormik.handleChange}>
                    </TextField>
                    <Typography>Unit</Typography>
                    <TextField 
                        select 
                        sx={{margin: '5px'}} 
                        value={ingredientFormik.values.unit}
                        id="unit"
                        name="unit"
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
                            handleAddIngredientMode() 
                            ingredientFormik.handleSubmit()}}
                    >
                        Confirm
                    </Button>
                    <Button 
                        onClick={() => {
                            handleAddIngredientMode()}}
                    >
                        Cancel
                    </Button>
                </Box> :
                  editIngredientMode ? 
                  <Box>
                    <Button>Delete</Button>
                    <Button>Move Up</Button>
                    <Button>Move Down</Button>
                    <Button onClick={() => handleEditIngredientMode()}>Cancel</Button>
                  </Box>:
                  <Box>
                    <Button onClick={() => handleAddIngredientMode()}>+ Add</Button>
                    <Button onClick={() => handleEditIngredientMode()}>Edit</Button>
                  </Box>
                
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
