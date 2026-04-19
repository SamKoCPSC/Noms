'use client'
import { useEffect, useState } from 'react'
import { Box, Button, Container, TextField, Typography, InputAdornment, Stack, IconButton } from "@mui/material";
import { Dancing_Script } from "next/font/google";
import { Search } from "@mui/icons-material";
import { signIn, useSession } from "next-auth/react";
import { useRouter } from "next/navigation";
import { useTheme } from "@emotion/react";
import useDebounce from '../hooks/useDebounce'
import { keyframes } from "@emotion/react";

const dancingScript = Dancing_Script({subsets: ['latin']})

const gradientAnimation = keyframes`
  0% {
    background-position: 0% 50%;
  }
  50% {
    background-position: 100% 50%;
  }
  100% {
    background-position: 0% 50%;
  }
`;

export default function HomeHeader() {
  const theme = useTheme()
  const {data: session, status} = useSession()
  const router = useRouter()
  const [searchTerm, setSearchTerm] = useState('')
  const debouncedSearchTerm = useDebounce(searchTerm, 300)

  const handleSearch = (event) => {
    event.preventDefault()
    router.push(`/search?name=${encodeURIComponent(searchTerm)}`)
  }

  useEffect(() => {
    if (!debouncedSearchTerm.trim()) return
    router.prefetch(`/search?name=${encodeURIComponent(debouncedSearchTerm)}`)
  }, [debouncedSearchTerm, router])

  return (
    <Box display={'flex'} flexDirection={'column'} alignItems={'center'} gap={'30px'}
    sx={{
      marginBottom: '75px', 
      position: 'relative',
      width: '100vw', 
      paddingY: '75px',
      clipPath: 'polygon(0 0, 100% 0, 100% 90%, 50% 100%, 0 90%)',
    }}>
      {/* Animated Gradient Background */}
      <Box
        sx={{
          position: 'absolute',
          top: 0,
          left: 0,
          right: 0,
          bottom: 0,
          background: 'linear-gradient(45deg, rgb(255, 200, 130), rgb(245, 240, 210), rgb(240, 225, 190), rgb(255, 200, 130))',
          backgroundSize: '400% 400%',
          animation: `${gradientAnimation} 5.33s ease infinite`,
          zIndex: 0,
        }}
      />
      
      {/* Content Layer */}
      <Box sx={{ position: 'relative', zIndex: 2, display: 'flex', flexDirection: 'column', alignItems: 'center', gap: '30px' }}>
        <Typography sx={{
          fontFamily: dancingScript.style.fontFamily,
          ":hover": {cursor: 'pointer'},
          [theme.breakpoints.up('575')]: {fontSize: '200px',},
          [theme.breakpoints.down('575')]: {fontSize: '150px',},
          [theme.breakpoints.down('420')]: {fontSize: '120px',},
          textShadow: '2px 2px 4px rgba(0, 0, 0, 0.1)', // Adding slight shadow for better readability
        }}>
          NOMS
        </Typography>
        <Typography sx={{
          fontSize: '18px', 
          marginTop: '-75px', 
          marginBottom: '20px',
          [theme.breakpoints.down('420')]: {fontSize: '16px',},
          textShadow: '1px 1px 2px rgba(0, 0, 0, 0.1)', // Adding slight shadow for better readability
        }}>
            Create, Share, and Manage Your Recipes
        </Typography>
        <form onSubmit={handleSearch}>
          <TextField 
            name="search"
            value={searchTerm}
            onChange={(event) => setSearchTerm(event.target.value)}
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
          <Stack direction={{ xs: "column", sm: "row" }} sx={{marginTop: '-15px'}} gap={{xs: 1, sm: 0}}>
            <Button variant="contained" onClick={() => router.push('/createRecipe')} sx={{borderRadius: '40px', width: '200px'}}>Create A Recipe</Button>
            <Button variant="contained" color="secondary" onClick={() => router.push('/myRecipes/' + session?.user?.id)} sx={{borderRadius: '40px', width: '200px'}}>Go To Your Recipes</Button>
          </Stack>
        }
      </Box>
    </Box>
  )
}