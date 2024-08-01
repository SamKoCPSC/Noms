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

  const formik = useFormik({
    initialValues: {
        title: ""
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
        
        <Box width='800px' sx={{}}> 
          <Typography fontSize={'50px'}>
            Create A Nom
          </Typography>
          <Divider sx={{margin: '10px'}}></Divider>
          <Stack direction='row'>
            <Typography fontSize="25px">Name:</Typography>
            <TextField variant="outlined" fullWidth></TextField>
          </Stack>
          <Box>
            <Typography fontSize="25px">Ingredients:</Typography>
            {addIngredientMode ? 
                <Box display="flex">
                    <Typography>Ingredient Name</Typography>
                    <TextField></TextField>
                    <Typography>Quantity</Typography>
                    <TextField></TextField>
                    <TextField select>
                        {units.map((unit) => (
                            <MenuItem key={unit} value={unit}>{unit}</MenuItem>
                        ))}
                    </TextField>
                </Box> : 
            <></>}
            <Button onClick={() => handleIngredientMode()}>{addIngredientMode ? 'Confirm' : '+ Add Ingredient'}</Button>
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
        </Box>
      </main>
    </Container>
    
  );
}
