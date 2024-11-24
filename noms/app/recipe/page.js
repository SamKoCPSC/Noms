'use client'

import { Box, Container, Typography } from "@mui/material"
import Navbar from '../components/Navbar'

export default function Recipe() {
    return (
        <Container
            sx={{
            marginTop: '65px',
            width: '70%',
            height: '100vh',
            justifyItems: 'center',
            backgroundColor: '#d1d1d1'
            }}
        >
            <Navbar></Navbar>
            <Typography sx={{justifySelf: 'center', fontSize: '50px'}}>Name of Recipe</Typography>
            <Typography sx={{justifySelf: 'center', fontSize: '20px'}}>Short Description of Recipe</Typography>
            <Box sx={{width: '600px', height: '350px', backgroundColor: '#efefef'}}></Box>
            
        </Container>
    )
}