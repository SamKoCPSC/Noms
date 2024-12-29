'use client'
import Image from "next/image";
import styles from "./page.module.css";
import * as React from 'react';
import Navbar from "./components/Navbar";
import Navdrawer from "./components/Navdrawer";
import RecipeCard from "./components/RecipeCard";
import {Box, Button, Container, TextField, Typography, InputAdornment} from "@mui/material";
import { Dancing_Script } from "next/font/google";
import { Search } from "@mui/icons-material";
import { SnackBarContext } from "./layout";
import axios from "axios";
import { useSession } from "next-auth/react";
import { useEffect } from "react";

const dancingScript = Dancing_Script({subsets: ['latin']})

function formatTimestamp(timestamp) {
    const isoTimestamp = timestamp.replace(" ", "T");
    const date = new Date(isoTimestamp);
    if (isNaN(date.getTime())) {
        throw new Error("Invalid PostgreSQL timestamp format.");
    }
    const options = {
        year: "numeric",
        month: "long",
        day: "numeric",
    };
    return date.toLocaleDateString(undefined, options);
}

export default function Home() {
  const {data: session, status} = useSession()
  const [randomRecipes, setRandomRecipes] = React.useState([])

  useEffect(() => {
    axios.post(
      '/api/getRecipes',
      {
        numOfResults: 10
      },
    ).then((response) => {
      console.log(response.data.result)
      setRandomRecipes(response.data.result)
    }).catch((error) => {
      console.log(error)
    })
  }, [])


  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <Typography sx={{marginTop: '75px', fontFamily: dancingScript.style.fontFamily, fontSize: '200px', ":hover": {cursor: 'pointer'}}}>
        NOMS
      </Typography>
      <Box display={'flex'} flexDirection={'column'} justifyItems={'center'} gap={'30px'}
      sx={{marginBottom: '50px', marginTop: '-20px'}}>
        <Typography sx={{fontSize: '18px'}}>Create, Share, and Manage Your Recipes</Typography>
        <TextField 
          variant="outlined"
          placeholder="Search for recipes"
          InputProps={{
            style: {
              backgroundColor: 'rgb(255, 255, 255)'
            },
            endAdornment: (
              <InputAdornment position="end">
                <Search />
              </InputAdornment>
            ),
          }}
          sx={{
            '& .MuiOutlinedInput-root': {
              borderRadius: '25px',
            }
          }}
        />
      </Box>
      <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
        {randomRecipes.map((recipe, index) => (
          <RecipeCard 
            key={index}
            title={recipe.name}
            description={recipe.description}
            author={recipe.author}
            date={formatTimestamp(recipe.datecreated)}
            ingredients={recipe.ingredients}
            instructions={recipe.instructions}
            additionalInfo={recipe.additionalInfo}
            imageURLs={recipe.imageurls}
          >
          </RecipeCard>
        ))}
      </Box>
    </Container>
    
  );
}
