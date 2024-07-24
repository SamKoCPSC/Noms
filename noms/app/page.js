import Image from "next/image";
import styles from "./page.module.css";
import {Box, Typography} from "@mui/material";
import { Dancing_Script } from "next/font/google";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function Home() {
  return (
    <main className={styles.main}>
      <Box> 
        <Typography sx={{fontFamily: dancingScript.style.fontFamily, fontSize: '150px'}}>
          NOMS
        </Typography>
      </Box>
    </main>
  );
}
