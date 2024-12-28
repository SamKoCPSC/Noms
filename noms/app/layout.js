'use client'
import React from "react";
import { Inter } from "next/font/google";
import "./globals.css";
import { ThemeProvider } from "@mui/material/styles";
import theme from "./theme";
import { SessionProvider } from "next-auth/react";
import { Box, Snackbar } from "@mui/material";
import Navbar from "./components/Navbar";

const inter = Inter({ subsets: ["latin"] });

export const SnackBarContext = React.createContext(null)

export default function RootLayout({ children }) {
  const [isSnackBarOpen, setSnackBarOpen] = React.useState(false)
  const [snackBarMessage, setSnackBarMessage] = React.useState("")
  const [snackBarVariant, setSnackBarVariant] = React.useState()
  const handleSnackBar = (message, variant) => {
    setSnackBarOpen(true)
    setSnackBarMessage(message)
    setSnackBarVariant(variant)
  }
  const closeSnackBar = () => {
    setSnackBarOpen(false)
  }
  return (
    <html lang="en">
      <body className={inter.className}>
        <Snackbar 
          open={isSnackBarOpen}
          autoHideDuration={6000}
          onClose={closeSnackBar}
          anchorOrigin={{vertical: 'top', horizontal: 'center'}}
          message={snackBarMessage}
          ContentProps={{sx: {background: snackBarVariant}}}
        />
        <SessionProvider>
          <ThemeProvider theme={theme}>
            <Box left='0%' width={'100%'}>
              <Navbar></Navbar>
            </Box>
            <SnackBarContext.Provider value={handleSnackBar}>{children}</SnackBarContext.Provider>
          </ThemeProvider>
        </SessionProvider>
      </body>
    </html>
  );
}
