'use client'
import Image from "next/image";
import styles from "./page.module.css";
import * as React from 'react';
import Navbar from "./components/Navbar";
import Navdrawer from "./components/Navdrawer";
import RecipeCard from "./components/RecipeCard";
import {Box, Container, Typography} from "@mui/material";
import { Dancing_Script } from "next/font/google";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function Home() {

  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <Box left='0%' width={'100%'}>
        <Navbar position='fixed'></Navbar>
      </Box>
      <Typography sx={{fontFamily: dancingScript.style.fontFamily, fontSize: '150px', ":hover": {cursor: 'pointer'}}}>
        NOMS
      </Typography>
      <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'20px'}}>
        {[...Array(100)].map((element, index) => (
          <RecipeCard key={index}></RecipeCard>
        ))}
      </Box>
    </Container>
    
  );
}
