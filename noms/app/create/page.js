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
  const [ingredients, setIngredients] = React.useState([{name: 'one', quantity: '1', unit: 'g'}, {name: 'two', quantity: '2', unit: 'g'}, {name: 'three', quantity: '3', unit: 'g'}, {name: 'four', quantity: '4', unit: 'g'}])
  const [selectedIngredients, setSelectedIngredients] = React.useState([])
  const [addInstructionMode, setAddInstructionMode] = React.useState(false)
  const [editInstructionMode, setEditInstructionMode] = React.useState(false)
  const [instructions, setInstructions] = React.useState([])
  const [selectedInstructions, setSelectedInstructions] = React.useState([])

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

  const instructionsFormik = useFormik({
    initialValues: {
      title: '',
      instruction: '',
    },
    onSubmit: (values, actions) => {
      setInstructions([...instructions, {title: values.title, instruction: values.instruction}])
      instructionsFormik.resetForm()
    },
  })

  const recipeFormik = useFormik({
    initialValues: {
        name: '',
        ingredients: [],
        instructions: [],
        notes: '',
    }
  })

  const handleAddIngredientMode = () => {
    setAddIngredientMode(!addIngredientMode)
  }

  const handleAddInstructionMode = () => {
    setAddInstructionMode(!addInstructionMode)
  }
  const handleEditIngredientMode = () => {
    if(editIngredientMode) {
      setSelectedIngredients(new Array(0))
    } else {
      setSelectedIngredients(new Array(ingredients.length).fill(false))
    }
    setEditIngredientMode(!editIngredientMode)
  }

  const handleEditInstructionMode = () => {
    if(editInstructionMode) {
      setSelectedInstructions(new Array(0))
    } else {
      setSelectedInstructions(new Array(instructions.length).fill(false))
    }
    setEditInstructionMode(!editInstructionMode)
  }
  
  const handleSelectedIngredients = (isSelected, ingredientIndex) => {
    setSelectedIngredients(selectedIngredients.map((element, index) => {
      if(index === ingredientIndex) {
        return isSelected
      }
      return element
    }))
  }

  const handleSelectedInstructions = (isSelected, instructionIndex) => {
  setSelectedInstructions(selectedInstructions.map((element, index) => {
      if(index === instructionIndex) {
        return isSelected
      }
      return element
    }))
  }

  const handleDeleteIngredients = () => {
    setIngredients(ingredients.filter((element, index) => {
      return !selectedIngredients[index]
    }))
    setSelectedIngredients(selectedIngredients.filter((element) => {
      return !element
    }))
  }

  const handleMoveUpIngredients = () => {
    let newIngredients = new Array(ingredients.length)
    newIngredients = structuredClone(ingredients)
    let newSelected = new Array(selectedIngredients.length)
    newSelected = structuredClone(selectedIngredients)
    for(let i = 1; i < newIngredients.length; i++) {
      if(selectedIngredients[i]) {
        let temp = newIngredients[i-1]
        newIngredients[i-1] = newIngredients[i]
        newIngredients[i] = temp
        
        temp = newSelected[i-1]
        newSelected[i-1] = newSelected[i]
        newSelected[i] = temp
      }
    }
    setIngredients(newIngredients)
    setSelectedIngredients(newSelected)
  }

  const handleMoveDownIngredients = () => {
    let newIngredients = new Array(ingredients.length)
    newIngredients = structuredClone(ingredients)
    let newSelected = new Array(selectedIngredients.length)
    newSelected = structuredClone(selectedIngredients)
    for(let i = newIngredients.length-2; i >= 0; i--) {
      if(selectedIngredients[i]) {
        let temp = newIngredients[i+1]
        newIngredients[i+1] = newIngredients[i]
        newIngredients[i] = temp
        
        temp = newSelected[i+1]
        newSelected[i+1] = newSelected[i]
        newSelected[i] = temp
      }
    }
    setIngredients(newIngredients)
    setSelectedIngredients(newSelected)
  }

  const handleDeleteInstructions = () => {
    setInstructions(instructions.filter((element, index) => {
      return !selectedInstructions[index]
    }))
    setSelectedInstructions(selectedInstructions.filter((element) => {
      return !element
    }))
  }

  const handleMoveUpInstructions = () => {
    let newInstructions = new Array(instructions.length)
    newInstructions = structuredClone(instructions)
    let newSelected = new Array(selectedInstructions.length)
    newSelected = structuredClone(selectedInstructions)
    for(let i = 1; i < newInstructions.length; i++) {
      if(selectedInstructions[i]) {
        let temp = newInstructions[i-1]
        newInstructions[i-1] = newInstructions[i]
        newInstructions[i] = temp
        
        temp = newSelected[i-1]
        newSelected[i-1] = newSelected[i]
        newSelected[i] = temp
      }
    }
    setInstructions(newInstructions)
    setSelectedInstructions(newSelected)
  }

  const handleMoveDownInstructions = () => {
    let newInstructions = new Array(instructions.length)
    newInstructions = structuredClone(instructions)
    let newSelected = new Array(selectedInstructions.length)
    newSelected = structuredClone(selectedInstructions)
    for(let i = newInstructions.length-2; i >= 0; i--) {
      if(selectedInstructions[i]) {
        let temp = newInstructions[i+1]
        newInstructions[i+1] = newInstructions[i]
        newInstructions[i] = temp
        
        temp = newSelected[i+1]
        newSelected[i+1] = newSelected[i]
        newSelected[i] = temp
      }
    }
    setInstructions(newInstructions)
    setSelectedInstructions(newSelected)
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
                <Box key={index}>
                  <Typography>
                    <Checkbox 
                      checked={selectedIngredients[index]}
                      sx={{padding: '0px'}}
                      onChange={(event) => {
                        handleSelectedIngredients(event.target.checked, index)
                      }}
                    >
                    </Checkbox>
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
                    <Button onClick={() => handleDeleteIngredients()}>Delete</Button>
                    <Button onClick={() => handleMoveUpIngredients()}>Move Up</Button>
                    <Button onClick={() => handleMoveDownIngredients()}>Move Down</Button>
                    <Button onClick={() => handleEditIngredientMode()}>Cancel</Button>
                  </Box>:
                  <Box>
                    <Button onClick={() => handleAddIngredientMode()}>+ Add</Button>
                    <Button onClick={() => handleEditIngredientMode()}>Edit</Button>
                  </Box>
                
            }
          </Box>
          <Box>
            <Typography fontSize={'25px'}>Instructions:</Typography>
            {instructions.map((instruction, index) => (
                editInstructionMode ? 
                <Box key={index}>
                  <Typography>
                    <Checkbox 
                      checked={selectedInstructions[index]}
                      sx={{padding: '0px'}}
                      onChange={(event) => {
                        handleSelectedInstructions(event.target.checked, index)
                      }}
                    >
                    </Checkbox>
                    {instruction.title + ' - ' + instruction.instruction}
                  </Typography>
                </Box> : 
                <Typography key={index} sx={{whiteSpace: 'pre-line'}}>{instruction.title + "\n" + instruction.instruction}</Typography>
            ))}
            {addInstructionMode ? 
                <Box display="flex" flexDirection='column'>
                  <Box display={'flex'}>
                    <Typography>Title</Typography>
                    <TextField 
                        sx={{margin: '5px'}} 
                        value={instructionsFormik.values.title}
                        id="title"
                        name="title"
                        onChange={instructionsFormik.handleChange}>
                    </TextField>
                  </Box>
                  <Typography>Instruction</Typography>
                  <TextField 
                        sx={{margin: '5px'}} 
                        value={instructionsFormik.values.instruction}
                        id="instruction"
                        name="instruction"
                        onChange={instructionsFormik.handleChange}
                        multiline>
                    </TextField>
                </Box> : 
                <></>
            }
            {addInstructionMode ? 
                  <Box>
                      <Button 
                          onClick={() => {
                              handleAddInstructionMode() 
                              instructionsFormik.handleSubmit()}}
                      >
                          Confirm
                      </Button>
                      <Button 
                          onClick={() => {
                              handleAddInstructionMode()}}
                      >
                          Cancel
                      </Button>
                  </Box> :
                    editInstructionMode ? 
                    <Box>
                      <Button onClick={() => handleDeleteInstructions()}>Delete</Button>
                      <Button onClick={() => handleMoveUpInstructions()}>Move Up</Button>
                      <Button onClick={() => handleMoveDownInstructions()}>Move Down</Button>
                      <Button onClick={() => handleEditInstructionMode()}>Cancel</Button>
                    </Box>:
                    <Box>
                      <Button onClick={() => handleAddInstructionMode()}>+ Add</Button>
                      <Button onClick={() => handleEditInstructionMode()}>Edit</Button>
                    </Box>
                  
              }
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
