import * as React from 'react';
import Navbar from "../components/Navbar";
import Navdrawer from "../components/Navdrawer";
import SearchResults from "../components/SearchResults";
import {Box, Container, CircularProgress} from "@mui/material";
import { Suspense } from "react";

export default function SearchPage({ searchParams }) {
  return (
    <Container maxWidth='false' sx={{justifyItems: 'center'}}>
      <Suspense fallback={<Box display="flex" justifyContent="center" alignItems="center" sx={{minHeight: '400px'}}><CircularProgress /></Box>}>
        <SearchResults searchParams={searchParams} />
      </Suspense>
    </Container>
  );
}
