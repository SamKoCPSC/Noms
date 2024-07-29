'use client'
import Image from "next/image";
import styles from "../page.module.css";
import * as React from 'react';
import Navbar from "../components/Navbar";
import Navdrawer from "../components/Navdrawer";
import {Box, Container, Typography} from "@mui/material";
import { Dancing_Script } from "next/font/google";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function Create() {
  const [isNavdrawerOpen, setNavdrawerOpen] = React.useState(false)

  const handleNavdrawerOpen = () => {
    setNavdrawerOpen(!isNavdrawerOpen)
  }

  return (
    <Container>
      <Box left='0%' width={'100%'}>
        <Navdrawer open={isNavdrawerOpen} setOpen={handleNavdrawerOpen}></Navdrawer>
        <Navbar setOpen={handleNavdrawerOpen} position='fixed' sx={{zIndex: (theme) => theme.zIndex.drawer + 1}}></Navbar>
      </Box>    
      <main className={styles.main}> 
        
        <Box> 
          <Typography sx={{fontFamily: dancingScript.style.fontFamily, fontSize: '150px'}}>
            NOMS
          </Typography>
        </Box>
      </main>
    </Container>
    
  );
}
