'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import { useFormik } from "formik";
import Navbar from '../components/Navbar'
import {Box, Container, Divider, Stack, TextField, Typography, Button} from "@mui/material";

export default function Create() {
  const formik = useFormik({
    initialValues: {
        title: ""
    }
  })

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
            <Button >+ Add Ingredient</Button>
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
