'use client'
import { Box, Button, Container, TextField, Typography, InputAdornment, Stack, IconButton } from "@mui/material";
import { Dancing_Script } from "next/font/google";
import { Search } from "@mui/icons-material";
import { signIn, useSession } from "next-auth/react";
import { useRouter } from "next/navigation";
import { useTheme } from "@emotion/react";

const dancingScript = Dancing_Script({subsets: ['latin']})

export default function HomeHeader() {
  const theme = useTheme()
  const {data: session, status} = useSession()
  const router = useRouter()

  const handleSearch = (event) => {
    event.preventDefault()
    const formData = new FormData(event.currentTarget)
    router.push(`/search?name=${formData.get('search')}`)
  }

  return (
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
          <Button variant="contained" onClick={() => router.push('/createRecipe')} sx={{borderRadius: '40px', width: '200px'}}>Create A Recipe</Button>
          <Button variant="contained" color="secondary" onClick={() => router.push('/create')} sx={{borderRadius: '40px', width: '200px'}}>Go To Your Recipes</Button>
        </Stack>
      }
    </Box>
  )
}
