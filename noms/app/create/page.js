'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import Navbar from '../components/Navbar'
import {Box, Container, Typography} from "@mui/material";
import { Dancing_Script } from "next/font/google";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function Create() {
  
  return (
    <Container>
      <Box left='0%' width={'100%'}>
        <Navbar></Navbar>
      </Box>
      <main className={styles.main}>
        
        <Box width='800px' sx={{backgroundColor: 'grey'}}> 
          <Typography fontSize={'50px'}>
            Create New Recipe
          </Typography>
        </Box>
      </main>
    </Container>
    
  );
}
