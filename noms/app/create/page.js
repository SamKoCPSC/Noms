'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import { useFormik } from "formik";
import axios from "axios";
import Navbar from '../components/Navbar'
import { Edit, Add, CloudUpload } from "@mui/icons-material";
import {Box, Container, Divider, Stack, TextField, Typography, Button, MenuItem, Checkbox, ImageList, ImageListItem} from "@mui/material";
import { styled } from "@mui/material";
import { useTheme } from "@emotion/react";
import { useSession } from "next-auth/react";

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
  const {data: session, status} = useSession()
  const [ingredients, setIngredients] = React.useState([])
  const [addIngredientMode, setAddIngredientMode] = React.useState(false)
  const [editIngredientMode, setEditIngredientMode] = React.useState(false)
  const [selectedIngredients, setSelectedIngredients] = React.useState([])

  const [instructions, setInstructions] = React.useState([])
  const [addInstructionMode, setAddInstructionMode] = React.useState(false)
  const [editInstructionMode, setEditInstructionMode] = React.useState(false)
  const [selectedInstructions, setSelectedInstructions] = React.useState([])

  const [images, setImages] = React.useState([])
  const [editImageMode, setEditImageMode] = React.useState(false)
  const [selectedImages, setSelectedImages] = React.useState([])

  const ingredientFormik = useFormik({
    initialValues: {
        quantity: '',
        unit: '', 
        name: ''
    },
    onSubmit: (values, actions) => {
        setIngredients([...ingredients, {quantity: values.quantity, unit: values.unit, name: values.name}])
        recipeFormik.setFieldValue("ingredients", [...ingredients, {quantity: values.quantity, unit: values.unit, name: values.name}])
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
      recipeFormik.setFieldValue('instructions', [...instructions, {title: values.title, instruction: values.instruction}])
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
        images: [],
    },
    onSubmit: async (values) => {
      let imageData = images.map((image) => {
        const formData = new FormData()
        formData.append('file', image);
        return formData;
      })
      axios.get(
        '/api/presignedURL', {
            params: {
            recipeName: recipeFormik.values.name,
            fileNames: imageData.map((image) => {return image.get('file').name})
          }
        },
      ).then((response) => {
          response.data.presignedURLs.forEach((presignedURL, index) => {
            axios.put(
              presignedURL,
              imageData[index].get('file'),
              {
                headers: {
                  "Content-Type": "image/jpeg"
                }
              }
            ).then(() => {
              console.log('s3 upload succeeded')
            }).catch(() => {
              console.log('s3 upload failed')
            })
          })         
      }).catch(() => {
          console.log('presigned URL failed')
      })
    }
  })

  const handleAddImages = (event) => {
    // const previewURLs = [...event.target.files].map((file) => {
    //   if(file) {
    //     return URL.createObjectURL(file);
    //   }
    // })
    // setImages([...images, ...previewURLs])
    setImages([...images, ...Array.from(event.target.files)])
    recipeFormik.setFieldValue("images", [...images, ...Array.from(event.target.files)])
  }

  const handleSelectedItems = (isSelected, itemIndex, selectedItems, setSelectedItems) => {
    setSelectedItems(selectedItems.map((element, index) => {
      if(index === itemIndex) {
        return isSelected
      }
      return element
    }))
  }

  const handleAddItemMode = (addItemMode, setAddItemMode) => {
    setAddItemMode(!addItemMode)
  }

  const handleEditItemMode = (item, editItemMode, setSelectedItems, setEditItemMode) => {
    if(editItemMode) {
      setSelectedItems(new Array(0))
    } else {
      setSelectedItems(new Array(item.length).fill(false))
    }
    setEditItemMode(!editItemMode)
  }

  const handleDeleteItems = (items, selectedItems, setItems, setSelectedItems, formikValue) => {
    let newItems = items.filter((element, index) => {
      return !selectedItems[index]
    })
    setItems(newItems)
    recipeFormik.setFieldValue(formikValue, newItems)
    setSelectedItems(selectedItems.filter((element) => {
      return !element
    }))
  }

  const handleMoveUpItems = (items, selectedItems, setItems, setSelectedItems, formikValue) => {
    let newItems = new Array(items.length)
    newItems = structuredClone(items)
    let newSelected = new Array(selectedItems.length)
    newSelected = structuredClone(selectedItems)
    for(let i = 1; i < newItems.length; i++) {
      if(selectedItems[i]) {
        let temp = newItems[i-1]
        newItems[i-1] = newItems[i]
        newItems[i] = temp
        
        temp = newSelected[i-1]
        newSelected[i-1] = newSelected[i]
        newSelected[i] = temp
      }
    }
    setItems(newItems)
    recipeFormik.setFieldValue(formikValue, newItems)
    setSelectedItems(newSelected)
  }

  const handleMoveDownItems = (items, selectedItems, setItems, setSelectedItems, formikValue) => {
    let newItems = new Array(items.length)
    newItems = structuredClone(items)
    let newSelected = new Array(selectedItems.length)
    newSelected = structuredClone(selectedItems)
    for(let i = newItems.length-2; i >= 0; i--) {
      if(selectedItems[i]) {
        let temp = newItems[i+1]
        newItems[i+1] = newItems[i]
        newItems[i] = temp
        
        temp = newSelected[i+1]
        newSelected[i+1] = newSelected[i]
        newSelected[i] = temp
      }
    }
    setItems(newItems)
    recipeFormik.setFieldValue(formikValue, newItems)
    setSelectedItems(newSelected)
  }

  const handleCreate = () => {

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
          <TextField 
            variant="outlined" 
            fullWidth
            name="name"
            id="name"
            value={recipeFormik.name}
            onChange={recipeFormik.handleChange}
          >
          </TextField>
          <Typography fontSize="30px">Description:</Typography>
          <SubText>Briefly describe your recipe. Include any interesting information or elements that make your recipe special.</SubText>
          <TextField 
            variant="outlined" 
            fullWidth 
            multiline 
            name="description"
            id="description"
            value={recipeFormik.description}
            onChange={recipeFormik.handleChange}
          >
          </TextField>
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
                        handleSelectedItems(event.target.checked, index, selectedIngredients, setSelectedIngredients)
                      }}
                    >
                    </Checkbox>
                    {ingredient.quantity + ingredient.unit + ' ' + ingredient.name}
                  </Typography>
                </Box> :
                <Stack key={index} direction={'row'} sx={{marginLeft: '8px'}}> 
                  <Typography sx={{margin: '4px', fontSize: '16px', fontWeight: '400'}}>{ingredient.quantity + ingredient.unit}</Typography>
                  <Typography sx={{margin: '4px', fontSize: '16px', fontWeight: '300'}}>{ingredient.name}</Typography>

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
                            handleAddItemMode(addIngredientMode, setAddIngredientMode) 
                            ingredientFormik.handleSubmit()}}
                    >
                        Confirm
                    </Button>
                    <Button 
                        variant="contained" color="warning"
                        onClick={() => {
                          handleAddItemMode(addIngredientMode, setAddIngredientMode)}}
                    >
                        Cancel
                    </Button>
                </Box> :
                  editIngredientMode ? 
                  <Box>
                    <Button variant="contained" color="error" onClick={() => handleDeleteItems(ingredients, selectedIngredients, setIngredients, setSelectedIngredients, "ingredients")}>Delete</Button>
                    <Button variant="contained" color="info" onClick={() => handleMoveUpItems(ingredients, selectedIngredients, setIngredients, setSelectedIngredients, "ingredients")}>Move Up</Button>
                    <Button variant="contained" color="info" onClick={() => handleMoveDownItems(ingredients, selectedIngredients, setIngredients, setSelectedIngredients, "ingredients")}>Move Down</Button>
                    <Button variant="contained" color="primary" onClick={() => handleEditItemMode(ingredients, editIngredientMode, setSelectedIngredients, setEditIngredientMode)}>Done</Button>
                  </Box>:
                  <Box>
                    <Button variant="contained" onClick={() => handleAddItemMode(addIngredientMode, setAddIngredientMode)}><Add/>Add</Button>
                    <Button disabled={ingredients.length === 0} variant="contained" color="secondary" onClick={() => handleEditItemMode(ingredients, editIngredientMode, setSelectedIngredients, setEditIngredientMode)}><Edit/>Edit</Button>
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
                        handleSelectedItems(event.target.checked, index, selectedInstructions, setSelectedInstructions)
                      }}
                    >
                    </Checkbox>
                    {instruction.title + ' - ' + instruction.instruction}
                  </Typography>
                </Box> : 
                <Stack key={index} sx={{marginLeft: '16px'}}>
                  <Typography sx={{whiteSpace: 'pre-line', marginTop: '8px', fontSize: '16px', fontWeight: '400'}}>{instruction.title}</Typography>
                  <Typography sx={{whiteSpace: 'pre-line', marginLeft: '8px', fontSize: '16px', fontWeight: '300'}}>{instruction.instruction}</Typography>
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
                              handleAddItemMode(addInstructionMode, setAddInstructionMode)
                              instructionsFormik.handleSubmit()}}
                      >
                          Confirm
                      </Button>
                      <Button 
                          variant="contained" color="warning"
                          onClick={() => {
                            handleAddItemMode(addInstructionMode, setAddInstructionMode)}}
                      >
                          Cancel
                      </Button>
                  </Box> :
                    editInstructionMode ? 
                    <Box>
                      <Button variant="contained" color="error" onClick={() => handleDeleteItems(instructions, selectedInstructions, setInstructions, setSelectedInstructions, "instructions")}>Delete</Button>
                      <Button variant="contained" color="info" onClick={() => handleMoveUpItems(instructions, selectedInstructions, setInstructions, setSelectedInstructions, "instructions")}>Move Up</Button>
                      <Button variant="contained" color="info" onClick={() => handleMoveDownItems(instructions, selectedInstructions, setInstructions, setSelectedInstructions, "instructions")}>Move Down</Button>
                      <Button variant="contained" color="primary" onClick={() => handleEditItemMode(instructions, editInstructionMode, setSelectedInstructions, setEditInstructionMode)}>Done</Button>
                    </Box>:
                    <Box>
                      <Button variant="contained" onClick={() => handleAddItemMode(addInstructionMode, setAddInstructionMode)}><Add/>Add</Button>
                      <Button disabled={instructions.length===0} variant="contained" color="secondary" onClick={() => handleEditItemMode(instructions, editInstructionMode, setSelectedInstructions, setEditInstructionMode)}><Edit/>Edit</Button>
                    </Box>
                  
              }
          </Box>
          <Divider sx={{margin: '30px'}}></Divider>
          <Box>
            <Typography fontSize='30px'>Notes:</Typography>
            <SubText>Add any non-instructional information, describe the recipe in more detail, and include your experiences. (Optional)</SubText>
            <TextField 
              variant="outlined" 
              fullWidth 
              multiline
              name="notes"
              id="notes"
              value={recipeFormik.notes}
              onChange={recipeFormik.handleChange}
            >
            </TextField>
          </Box>
          <Divider sx={{margin: '30px'}}></Divider>
          <Typography fontSize="30px">Images</Typography>
          <SubText>Include images of your recipe. The first image will be the main thumbnail that can be seen when searched.</SubText>
          <Box>
            {editImageMode ?
            <Box>
              <Button variant="contained" color="error" onClick={() => handleDeleteItems(images, selectedImages, setImages, setSelectedImages, "images")}>Delete</Button>
              <Button variant="contained" color="info" onClick={() => handleMoveUpItems(images, selectedImages, setImages, setSelectedImages, "images")}>Move Up</Button>
              <Button variant="contained" color="info" onClick={() => handleMoveDownItems(images, selectedImages, setImages, setSelectedImages, "images")}>Move Down</Button>
              <Button variant="contained" color="warning" onClick={() => handleEditItemMode(images, editImageMode, setSelectedImages, setEditImageMode)}>Done</Button>
            </Box> 
            : 
              <Box>
                <Button
                  component="label"
                  role={undefined}
                  variant="contained"
                  tabIndex={-1}
                  startIcon={<CloudUpload/>}
                >
                  Upload
                  <VisuallyHiddenInput
                    type="file"
                    onChange={(event) => handleAddImages(event)}
                    multiple
                  />
                </Button>
                <Button disabled={images.length===0} variant="contained" color="secondary" onClick={() => handleEditItemMode(images, editImageMode, setSelectedImages, setEditImageMode)}><Edit/>Edit</Button>
              </Box>
            }
          </Box>
          <ImageList cols={4} gap={8} sx={{marginTop: '20px'}}>
              {images.map((image, index) => {
                const previewURL = URL.createObjectURL(image)
                return (
                  <ImageListItem key={index}>
                    {editImageMode ?
                      <Checkbox 
                        checked={selectedImages[index]}
                        sx={{
                          position: 'absolute',
                          color: 'white',
                        }}
                        onChange={(event) => {
                          handleSelectedItems(event.target.checked, index, selectedImages, setSelectedImages)
                        }}
                      />
                    :
                      <Box/>
                    }
                    <img
                      src={previewURL}
                      loading="lazy"
                      style={{width: '100%', height: '120px'}}
                    />
                  
                  </ImageListItem>
                )
              })}
          </ImageList>
          <Divider sx={{margin: '30px'}}></Divider>
          <Stack direction={'row'} sx={{justifyContent: 'end'}}>
            <Button variant="contained" color="error" sx={{top: '30px'}}>Cancel</Button>
            <Button variant="contained" color="secondary" sx={{top: '30px'}}>Save</Button>
            <Button disabled={status !== "authenticated"} variant="contained" sx={{top: '30px'}} onClick={() => {recipeFormik.handleSubmit()}}>Create</Button>
          </Stack>
        </Box>
      </main>
    </Container>
    
  );
}
