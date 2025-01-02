import { Typography, Container, Box, Divider } from "@mui/material";

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
    const userData = await getUserData(params.id)

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
                <Typography fontSize={'60px'}>Profile</Typography>
                <Divider sx={{marginY: '30px'}}/>
                <Typography fontSize='1.375rem'>Name: {userData.name}</Typography>
                <Typography fontSize='1.375rem'>Email: {userData.email}</Typography>
                <Typography fontSize='1.375rem'>User ID: {userData.id}</Typography>
                <Typography fontSize='1.375rem'>Account Created: {userData.datecreated}</Typography>
                <Typography fontSize='1.375rem'>Headline</Typography>
                <Typography fontSize='1.375rem'>Bio</Typography>
                </Box>
            </Box>
        </Container>
    )
}