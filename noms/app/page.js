'use client'
import Image from "next/image";
import styles from "./page.module.css";
import * as React from 'react';
import Navbar from "./components/Navbar";
import Navdrawer from "./components/Navdrawer";
import RecipeCard from "./components/RecipeCard";
import {Box, Button, Container, TextField, Typography, InputAdornment, Stack, IconButton} from "@mui/material";
import { Dancing_Script } from "next/font/google";
import { Search } from "@mui/icons-material";
import { SnackBarContext } from "./layout";
import axios from "axios";
import { signIn, useSession } from "next-auth/react";
import { useRouter } from "next/navigation";
import { useEffect } from "react";
import { useTheme } from "@emotion/react";
import Carousel from "./components/Carousel";

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
  const theme = useTheme()
  const {data: session, status} = useSession()
  const router = useRouter()
  const [randomRecipes, setRandomRecipes] = React.useState([])

  useEffect(() => {
    axios.post(
      '/api/getRecipes',
      {
        numOfResults: 24
      },
    ).then((response) => {
      setRandomRecipes(response.data.result)
    }).catch((error) => {
      console.log(error)
    })
  }, [])

  const handleSearch = (event) => {
    event.preventDefault()
    const formData = new FormData(event.currentTarget)
    router.push(`/search?name=${formData.get('search')}`)
  }

  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <Box display={'flex'} flexDirection={'column'} alignItems={'center'} gap={'30px'}
      sx={{
        marginBottom: '75px', 
        background: 'linear-gradient(to bottom right, rgb(250, 215, 160), rgb(240, 238, 225))', 
        width: '100vw', 
        paddingY: '75px',
        clipPath: 'polygon(0 0, 100% 0, 100% 90%, 50% 100%, 0 90%)'
      }}>
        <Typography sx={{
          fontFamily: dancingScript.style.fontFamily,
          ":hover": {cursor: 'pointer'},
          [theme.breakpoints.up('575')]: {fontSize: '200px',},
          [theme.breakpoints.down('575')]: {fontSize: '150px',},
          [theme.breakpoints.down('420')]: {fontSize: '120px',},
          }}>
          NOMS
        </Typography>
        <Typography sx={{
          fontSize: '18px', 
          marginTop: '-75px', 
          marginBottom: '20px',
          [theme.breakpoints.down('420')]: {fontSize: '16px',},
          }}>
            Create, Share, and Manage Your Recipes
        </Typography>
        <form onSubmit={handleSearch}>
          <TextField 
            name="search"
            variant="outlined"
            placeholder="Search for recipes"
            InputProps={{
              style: {
                backgroundColor: 'rgb(255, 255, 255)'
              },
              endAdornment: (
                <InputAdornment position="end">
                  <IconButton type="submit">
                    <Search />
                  </IconButton>
                </InputAdornment>
              ),
            }}
            sx={{
              '& .MuiOutlinedInput-root': {borderRadius: '25px',},
              width: '500px',
              [theme.breakpoints.down('525')]: {width: '90vw',},
            }}
          />
        </form>
        {status === 'unauthenticated' &&
          <Button variant="contained" onClick={() => signIn('google')} sx={{borderRadius: '40px', width: '200px', marginTop: '-15px'}}>Login or Sign-up</Button>
        }
        {status === 'authenticated' &&
          <Stack direction={{ xs: "column", sm: "row" }} sx={{marginTop: '-15px'}}>
            <Button variant="contained" onClick={() => router.push('/create')} sx={{borderRadius: '40px', width: '200px'}}>Create A Recipe</Button>
            <Button variant="contained" color="secondary" onClick={() => router.push('/create')} sx={{borderRadius: '40px', width: '200px'}}>Go To Your Recipes</Button>
          </Stack>
        }
      </Box>
      <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
        {randomRecipes.map((recipe, index) => (
          <RecipeCard 
            key={index}
            id={recipe.id}
            name={recipe.name}
            description={recipe.description}
            author={recipe.author}
            date={formatTimestamp(recipe.datecreated)}
            ingredients={recipe.ingredients}
            instructions={recipe.instructions}
            additionalInfo={recipe.additionalinfo}
            imageURLs={recipe.imageurls}
            status={recipe.status}
            baseid={recipe.baseid}
            version={recipe.version}
            branchid = {recipe.branchid}
            branchbase = {recipe.branchbase}
          >
          </RecipeCard>
        ))}
      </Box>
    </Container>
    
  );
}
