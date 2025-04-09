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
import { Suspense } from "react";

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
  const searchParams = useSearchParams()
  const [recipes, setRecipes] = React.useState([])

  useEffect(() => {
    const name = searchParams.get('name')
    const ingredients = searchParams.get('ingredients')
    fetch(
      `/api/search?name=${name}&ingredients=${ingredients}`
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
      <Box display={'flex'} flexDirection={'row'} flexWrap={'wrap'} sx={{justifyContent: 'center', gap:'40px'}}>
          {recipes.map((recipe, index) => { 
              if(recipe.status === 'public') {
                  return (
                      <RecipeCard
                          key={index}
                          id={recipe.recipeid}
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
                          notes={recipe.notes}
                          branchid={recipe.branchid}
                          branchbase={recipe.branchbase}
                      />
                  )
              }  
          })}
      </Box>
    </Container>
    
  );
}
