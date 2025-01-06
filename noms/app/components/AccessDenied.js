'use client'
import { Box, Stack, Typography, Button } from "@mui/material";
import theme from "../theme";
import { ErrorOutline } from "@mui/icons-material";
import { signIn } from "next-auth/react";

export default function AccessDenied() {
    return (
        <Box
            sx={{
                display: "flex",
                flexDirection: "column",
                alignItems: "center",
                justifyContent: "center",
                justifySelf: 'center',
                width: '50%',
                height: '120px',
                backgroundColor: "white",
                textAlign: "center",
                px: 2,
                marginTop: '200px',
                borderColor: theme.palette.error.main,
                borderStyle: 'solid',
                borderWidth: 3,
                boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
                borderRadius: '25px',
            }}
        >
            <Stack direction={'row'} sx={{alignItems: 'center', gap: '5px'}}>
                <ErrorOutline sx={{color: theme.palette.error.main}}/>
                <Typography sx={{fontSize: '1.75rem'}}>Please login to the correct account to access this page</Typography>
            </Stack>
            <Button variant="contained" onClick={() => signIn('google')} sx={{borderRadius: '40px', width: '100px'}}>Login</Button>
        </Box>
    )
}