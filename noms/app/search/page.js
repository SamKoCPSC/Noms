'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import Navbar from "../components/Navbar";
import Navdrawer from "../components/Navdrawer";
import RecipeCard from "../components/RecipeCard";
import {Box, Container, Typography} from "@mui/material";
import { Dancing_Script } from "next/font/google";
import { useSearchParams } from "next/navigation";
import { useEffect } from "react";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function Home() {
  const searchParams = useSearchParams()
  const [recipes, setRecipes] = React.useState([])

  useEffect(() => {
    const name = searchParams.get('name')
    fetch(
      `/api/search?name=fries`
    ).then((response) => {
        if(!response.ok) {
            console.error(response)
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        setRecipes(data.result)
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
  }, [searchParams])

  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <Typography sx={{marginTop: '100px'}}>{JSON.stringify(recipes)}</Typography>
      {/* <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'20px'}}>
        {[...Array(100)].map((element, index) => (
          <RecipeCard key={index}></RecipeCard>
        ))}
      </Box> */}
    </Container>
    
  );
}
