import * as React from 'react';
import { Suspense } from "react";
import HomeHeader from "./components/HomeHeader";
import RecipesDisplay from "./components/RecipesDisplay";
import {Box, Container, CircularProgress} from "@mui/material";

export default function Home() {
  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <HomeHeader />
      <Suspense fallback={<Box display="flex" justifyContent="center" alignItems="center" sx={{minHeight: '400px'}}><CircularProgress /></Box>}>
        <RecipesDisplay />
      </Suspense>
    </Container>
  );
}
