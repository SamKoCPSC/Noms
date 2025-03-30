import { Typography, Container, Box, Divider } from "@mui/material";
import AccessDenied from "@/app/components/AccessDenied";
import { styled } from "@mui/material";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/api/auth/[...nextauth]/route";
import { useTheme } from "@emotion/react";

export async function generateStaticParams() {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT *
                FROM users
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((response) => {
        return response.result.map((user) => ({
            userID: user.id.toString()
        }))
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

async function getUserData(id) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT * 
                FROM users 
                WHERE id = %s;
            `,
            values: [id]
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((response) => {
        return response.result[0]
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

function formatTimestamp(timestamp) {
    const isoTimestamp = timestamp.replace(" ", "T");
    const date = new Date(isoTimestamp);
    if (isNaN(date.getTime())) {
        throw new Error("Invalid PostgreSQL timestamp format.");
    }
    const options = {
        year: "numeric",
        month: "long",
        day: "numeric",
    };
    return date.toLocaleDateString(undefined, options);
}

export default async function({ params }) {
    const userData = await getUserData(params.userID)
    const session = await getServerSession(authOptions)

    const textStyle = {
        recipeTitleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    if(!session || userData.email !==  session.user.email) {
        return (
            <AccessDenied/>
        )
    }

    return (
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box display={'flex'} flexDirection={'column'} flexWrap={'wrap'} sx={{width: {md: '800px', sm: '100%', xs: '100%'}, alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Box width={'100%'} display={"flex"} flexDirection={'column'}
                    sx={{
                    borderColor: 'rgb(230, 228, 215)',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)',
                    // gap: '20px', 
                    backgroundColor: 'white',
                    borderRadius: '30px',
                    padding: '40px'
                    }}
                > 
                <Typography fontSize={{sm: '4rem', xs: '3rem'}}>Your Account</Typography>
                <Divider sx={{marginY: '30px'}}/>
                <Typography sx={{fontSize: '0.68rem', color: 'gray'}}>Your display name which appears on your public profile and can be changed at any time</Typography>
                <Typography fontSize='1.5rem'>Name: {userData.name}</Typography>
                <Divider sx={{marginY: '30px'}}/>
                <Typography fontSize='2rem' >Account Information</Typography>
                <Typography sx={{fontSize: '0.68rem', color: 'gray', marginBottom: '15px'}}>Emails and User IDs are unique identifiers for your account and cannot be changed</Typography>
                <Typography fontSize='1.5rem'>Email: {userData.email}</Typography>
                <Typography fontSize='1.5rem'>User ID: {userData.id}</Typography>
                <Typography fontSize='1.5rem'>Account Created: {formatTimestamp(userData.datecreated)}</Typography>
                <Divider sx={{marginY: '30px'}}/>
                <Typography fontSize='2rem' >Profile</Typography>
                <Typography sx={{fontSize: '0.68rem', color: 'gray', marginBottom: '15px'}}>Content that appears on your public profile page</Typography>
                <Typography fontSize='1.5rem'>Image</Typography>
                <Typography fontSize='1.5rem'>Headline</Typography>
                <Typography fontSize='1.5rem'>Bio</Typography>
                </Box>
            </Box>
        </Container>
    )
}