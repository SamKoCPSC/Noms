'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import { useFormik } from "formik";
import Navbar from '../components/Navbar'
import {Box, Container, Divider, Typography} from "@mui/material";

export default function Create() {
  const formik = useFormik({
    initialValues: {
        

    }
  })

  return (
    <Container>
      <Box left='0%' width={'100%'}>
        <Navbar></Navbar>
      </Box>
      <main className={styles.main}>
        
        <Box width='800px' sx={{backgroundColor: 'green'}}> 
          <Typography fontSize={'50px'}>
            Create New Recipe
          </Typography>
          <Divider></Divider>
          <Typography>Title:</Typography>
          <Typography>Ingredients:</Typography>
          <Typography>Instructions:</Typography>
          <Divider></Divider>
          <Typography>Notes:</Typography>
        </Box>
      </main>
    </Container>
    
  );
}
