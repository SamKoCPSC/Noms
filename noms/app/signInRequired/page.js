'use client'

import { Typography, Box, Stack, Button, } from "@mui/material";
import { ErrorOutline } from "@mui/icons-material";
import { signIn } from "next-auth/react";
import theme from "../theme";
import { useRouter } from "next/navigation";

export default async function() {
    const router = useRouter()

    return (
        <Box
            sx={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                justifySelf: 'center',
                width: '50%',
                height: '200px',
                backgroundColor: "white",
                textAlign: "center",
                px: 2,
                marginTop: '200px',
                borderColor: theme.palette.primary.main,
                borderStyle: 'solid',
                borderWidth: 3,
                boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
                borderRadius: '25px',
            }}
        >
            <Stack direction={'row'} sx={{alignItems: 'center', gap: '5px'}}>
                <ErrorOutline sx={{color: theme.palette.primary.main}}/>
                <Typography sx={{fontSize: '1.75rem'}}>An account is required to access the requested page. Please login or create a new account.</Typography>
            </Stack>
            <Stack direction={'row'} sx={{alignItems: 'center', gap: '5px'}}>
                <Button variant="contained" onClick={() => signIn('google')} sx={{borderRadius: '40px', width: '100px'}}>Login</Button>
                <Button variant="contained" color="secondary" onClick={() => router.push('/')} sx={{borderRadius: '40px', width: '100px'}}>Home</Button>
            </Stack>
        </Box>
    )
}