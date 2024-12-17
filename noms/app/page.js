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

const recipeData = {
  title: 'Croissant',
  description: 'This classic croissant recipe produces golden, buttery, and flaky pastries that melt in your mouth. Perfectly laminated layers of dough are crafted with patience and care, filled with rich butter, and baked to perfection. Enjoy these delectable croissants fresh from the oven as a breakfast treat or a delightful snack.',
  author: 'Sam Ko',
  date: "December 6, 2024",
  ingredients: [
    {name: 'Flour', unit: 'g', amount: '400'},
    {name: 'Water', unit: 'g', amount: '160'},
    {name: 'Salt', unit: 'g', amount: '8'},
    {name: 'Yeast', unit: 'g', amount: '12'},
    {name: 'Sugar', unit: 'g', amount: '45'}
  ],
  instructions: [
    {step: 'Prepate the Dough', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
    {step: 'Prepare the Butter', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
    {step: 'Laminate the Dough', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
    {step: 'Shape the Croissants', details: ["Mix flour, sugar, salt, yeast, and milk to form a soft dough. Knead until smooth", "Cover and let it rise until it doubles in size"]},
  ]
}

  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <Box left='0%' width={'100%'}>
        <Navbar position='fixed'></Navbar>
      </Box>
      <Typography sx={{marginTop: '50px', fontFamily: dancingScript.style.fontFamily, fontSize: '150px', ":hover": {cursor: 'pointer'}}}>
        NOMS
      </Typography>
      <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'20px'}}>
        {[...Array(100)].map((element, index) => (
          <RecipeCard 
            key={index}
            title={recipeData.title}
            description={recipeData.description}
            author={recipeData.author}
            date={recipeData.date}
            ingredients={recipeData.ingredients}
            instructions={recipeData.instructions}
          >
          </RecipeCard>
        ))}
      </Box>
    </Container>
    
  );
}
