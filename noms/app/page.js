import Image from "next/image";
import styles from "./page.module.css";
import Navbar from "./components/Navbar"
import {Box, Container, Typography} from "@mui/material";
import { Dancing_Script } from "next/font/google";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function Home() {
  return (
    <Container>
      <Box left='0%' position={'fixed'} width={'100%'}>
        <Navbar></Navbar>
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
