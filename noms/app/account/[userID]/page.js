import { Typography, Container, Box, Divider } from "@mui/material";
import { styled } from "@mui/material";

export async function generateStaticParams() {
    const userIDs = ['1']
    return userIDs.map((id) => {
      return {userID: id}
    });
}

async function getUserData(id) {
    return fetch(
        `http://localhost:3000/api/getAccount?id=${id}`
    ).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result[0]
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
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

    const textStyle = {
        recipeTitleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }
    return (
        <Container maxWidth='false' sx={{justifyItems: 'center', width: '70%'}}>
            <Box display={'flex'} flexDirection={'column'} flexWrap={'wrap'} sx={{width: '100%',alignItems: 'center', gap:'40px', marginTop: '100px'}}>
                <Box width='800px' display={"flex"} flexDirection={'column'}
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
                <Typography fontSize={'4rem'}>Your Account</Typography>
                <Divider sx={{marginY: '30px'}}/>
                <Typography sx={{fontSize: '0.68rem', color: 'gray'}}>Your display name which appears on your public profile and can be changed at any time</Typography>
                <Typography fontSize='1.5rem'>Name: {userData.name}</Typography>
                <Divider sx={{marginTop: '30px'}}/>
                <Typography fontSize='2rem' >Account Information</Typography>
                <Typography sx={{fontSize: '0.68rem', color: 'gray', marginBottom: '15px'}}>Emails and User IDs are unique identifiers for your account and cannot be changed</Typography>
                <Typography fontSize='1.5rem'>Email: {userData.email}</Typography>
                <Typography fontSize='1.5rem'>User ID: {userData.id}</Typography>
                <Typography fontSize='1.5rem'>Account Created: {formatTimestamp(userData.datecreated)}</Typography>
                <Divider sx={{marginTop: '30px'}}/>
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