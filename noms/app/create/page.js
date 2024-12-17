'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import { useFormik } from "formik";
import axios from "axios";
import Navbar from '../components/Navbar'
import EditIcon from '@mui/icons-material/Edit';
import { CloudUpload } from "@mui/icons-material";
import {Box, Container, Divider, Stack, TextField, Typography, Button, MenuItem, Checkbox} from "@mui/material";
import { styled } from "@mui/material";
import { useTheme } from "@emotion/react";

const VisuallyHiddenInput = styled('input')({
  clip: 'rect(0 0 0 0)',
  clipPath: 'inset(50%)',
  height: 1,
  overflow: 'hidden',
  position: 'absolute',
  bottom: 0,
  left: 0,
  whiteSpace: 'nowrap',
  width: 1,
});

const SubText = styled('p')( {
  fontSize: '11px',
  color: 'grey',
})

const units = ['g', 'mL']

export default function Create() {
  const theme = useTheme()
  const [addIngredientMode, setAddIngredientMode] = React.useState(false)
  const [editIngredientMode, setEditIngredientMode] = React.useState(false)
  const [ingredients, setIngredients] = React.useState([])
  const [selectedIngredients, setSelectedIngredients] = React.useState([])
  const [addInstructionMode, setAddInstructionMode] = React.useState(false)
  const [editInstructionMode, setEditInstructionMode] = React.useState(false)
  const [instructions, setInstructions] = React.useState([])
  const [selectedInstructions, setSelectedInstructions] = React.useState([])
  const [selectedImage, setSelectedImage] = React.useState()

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
        description: '',
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

  const handleFileUpload = async (event) => {
    const file = event.target.files[0];
    const formData = new FormData();
    formData.append('image', file);
    console.log(formData)
  }
  return (
    <Container>
      <Box left='0%' width={'100%'}>
        <Navbar></Navbar>
      </Box>
      <main className={styles.main}>
        
        <Box width='800px' display={"flex"} flexDirection={'column'}
        sx={{
          borderColor: 'rgb(230, 228, 215)',
          borderStyle: 'solid',
          borderWidth: 2,
          boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
          // gap: '20px', 
          backgroundColor: 'white',
          borderRadius: '30px',
          padding: '40px'
        }}> 
          <Typography fontSize={'60px'}>
            Create A Recipe
          </Typography>
          <Divider sx={{marginY: '30px'}}></Divider>
          <Typography fontSize="30px">Name:</Typography>
          <SubText>Give your recipe an identifiable name. The name will be used for searches and does not have to be unique.</SubText>
          <TextField variant="outlined" fullWidth sx={{margin: '5px'}}></TextField>
          <Typography fontSize="30px">Description:</Typography>
          <SubText>Briefly describe your recipe. Include any interesting information or elements that make your recipe special.</SubText>
          <TextField variant="outlined" fullWidth multiline sx={{margin: '5px'}}></TextField>
          <Divider sx={{marginY: '30px'}}></Divider>
          <Box>
            <Typography fontSize="30px">Ingredients:</Typography>
            <SubText>Specify each ingredient's quantity along with it's unit of measurement. Additional recipe iterations can be added later.</SubText>  
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
                <Stack direction={'row'} sx={{marginLeft: '8px'}}> 
                  <Typography key={index} sx={{margin: '4px', fontSize: '16px', fontWeight: '400'}}>{ingredient.quantity + ingredient.unit}</Typography>
                  <Typography key={index} sx={{margin: '4px', fontSize: '16px', fontWeight: '300'}}>{ingredient.name}</Typography>

                </Stack> 
            ))}
            {addIngredientMode ? 
                <Box display="flex" sx={{gap: '8px'}}>
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
                        variant="contained" color="primary"
                        onClick={() => {
                            handleAddIngredientMode() 
                            ingredientFormik.handleSubmit()}}
                    >
                        Confirm
                    </Button>
                    <Button 
                        variant="contained" color="warning"
                        onClick={() => {
                            handleAddIngredientMode()}}
                    >
                        Cancel
                    </Button>
                </Box> :
                  editIngredientMode ? 
                  <Box>
                    <Button variant="contained" color="error" onClick={() => handleDeleteIngredients()}>Delete</Button>
                    <Button variant="contained" color="info" onClick={() => handleMoveUpIngredients()}>Move Up</Button>
                    <Button variant="contained" color="info" onClick={() => handleMoveDownIngredients()}>Move Down</Button>
                    <Button variant="contained" color="primary" onClick={() => handleEditIngredientMode()}>Done</Button>
                  </Box>:
                  <Box>
                    <Button variant="contained" color="secondary" onClick={() => handleAddIngredientMode()}>+ Add</Button>
                    <Button variant="contained" color="secondary" onClick={() => handleEditIngredientMode()}>Edit</Button>
                  </Box>
                
            }
          </Box>
          <Box>
            <Typography fontSize={'30px'}>Instructions:</Typography>
            <SubText>Give each step a descriptive title and describe in detail the instructions required to replicate the recipe.</SubText>
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
                <Stack sx={{marginLeft: '16px'}}>
                  <Typography key={index} sx={{whiteSpace: 'pre-line', marginTop: '8px', fontSize: '16px', fontWeight: '400'}}>{instruction.title}</Typography>
                  <Typography key={index} sx={{whiteSpace: 'pre-line', marginLeft: '8px', fontSize: '16px', fontWeight: '300'}}>{instruction.instruction}</Typography>
                </Stack>
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
                          variant="contained" color="primary"
                          onClick={() => {
                              handleAddInstructionMode() 
                              instructionsFormik.handleSubmit()}}
                      >
                          Confirm
                      </Button>
                      <Button 
                          variant="contained" color="warning"
                          onClick={() => {
                              handleAddInstructionMode()}}
                      >
                          Cancel
                      </Button>
                  </Box> :
                    editInstructionMode ? 
                    <Box>
                      <Button variant="contained" color="error" onClick={() => handleDeleteInstructions()}>Delete</Button>
                      <Button variant="contained" color="info" onClick={() => handleMoveUpInstructions()}>Move Up</Button>
                      <Button variant="contained" color="info" onClick={() => handleMoveDownInstructions()}>Move Down</Button>
                      <Button variant="contained" color="primary" onClick={() => handleEditInstructionMode()}>Done</Button>
                    </Box>:
                    <Box>
                      <Button variant="contained" color="secondary" onClick={() => handleAddInstructionMode()}>+ Add</Button>
                      <Button variant="contained" color="secondary" onClick={() => handleEditInstructionMode()}>Edit</Button>
                    </Box>
                  
              }
          </Box>
          <Divider sx={{margin: '30px'}}></Divider>
          <Box>
            <Typography fontSize='30px'>Notes:</Typography>
            <SubText>Add any non-instructional information, describe the recipe in more detail, and include your experiences. (Optional)</SubText>
            <TextField variant="outlined" fullWidth multiline></TextField>
          </Box>
          <Divider sx={{margin: '30px'}}></Divider>
          <Typography fontSize="30px">Images</Typography>
          <SubText>Include images of your recipe. The first image will be the main thumbnail that can be seen when searched.</SubText>
          <Button
            component="label"
            role={undefined}
            variant="contained"
            tabIndex={-1}
            startIcon={<CloudUpload/>}
            sx={{width: '150px'}}
          >
            Upload Images
            <VisuallyHiddenInput
              type="file"
              onChange={(event) => handleFileUpload(event)}
              multiple
            />
          </Button>
          <Divider sx={{margin: '30px'}}></Divider>
          <Stack direction={'row'} sx={{justifyContent: 'end'}}>
            <Button variant="contained" color="error" sx={{top: '30px'}}>Cancel</Button>
            <Button variant="contained" color="secondary" sx={{top: '30px'}}>Save</Button>
            <Button variant="contained" sx={{top: '30px'}}>Create</Button>
          </Stack>
        </Box>
      </main>
    </Container>
    
  );
}
