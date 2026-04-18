import RecipeCard from "@/app/components/RecipeCard";
import { Container, Typography, Box, Divider, Button } from "@mui/material";
import AccessDenied from "@/app/components/AccessDenied";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/api/auth/[...nextauth]/route";
import Link from "next/link";

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
    }).then((data) => {
        return data.result.map((user) => ({
            userID: user.id.toString()
        }))
    })
    .catch((error) => {
        console.error(error)
        return {message: 'error'}
    })
}

async function getUserProjectData(id) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT 
                    p.id AS id,
                    p.name AS name,
                    p.description,
                    p.created_at AS datecreated,
                    u.name AS author,
                    u.email
                FROM 
                    projects p
                JOIN 
                    users u ON p.ownerid = u.id
                WHERE 
                    u.id = %s;
            `,
            values: [id]
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result
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
    const userProjects = await getUserProjectData(params.userID)
    const session = await getServerSession(authOptions)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    if(!session || params.userID !== session.user.id.toString()) {
        return (
            <AccessDenied/>
        )
    }

    return(
        <Container maxWidth='false' sx={{justifyItems: 'center'}}>
            <Box 
                display="flex"
                alignItems="flex-start"
                sx={{
                    width: '100%',
                    backgroundColor: 'white',
                    padding: '20px',
                    margin: '30px',
                    borderRadius: '15px',
                    borderColor: 'rgb(230, 228, 215',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
                }}
            >
                <Box sx={{ flex: 1 }}>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.titleSize,
                            marginBottom: '0px',
                            textAlign: 'left'
                        }}
                    >
                        {"My Projects"}
                    </Typography>
                </Box>
                <Box 
                    display="flex" 
                    flexDirection="row" 
                    alignItems="flex-end"
                    sx={{ gap: '15px' }}
                >
                    <Box display="flex" flexDirection="column" alignItems="center" sx={{ minWidth: '80px' }}>
                        <Typography 
                            variant="h4" 
                            sx={{ 
                                fontSize: textStyle.sectionTitleSize,
                                fontWeight: 'bold',
                                color: 'secondary.main'
                            }}
                        >
                            {userProjects.length || 0}
                        </Typography>
                        <Typography 
                            variant="body2" 
                            sx={{ 
                                fontSize: textStyle.paragraphSize,
                                color: 'text.secondary'
                            }}
                        >
                            Projects
                        </Typography>
                        <Link href={`/createRecipe`}>
                            <Button variant="contained">
                                Create Project
                            </Button>
                        </Link>
                    </Box>
                </Box>
            </Box>

            <Box 
                display="flex"
                flexDirection={'column'}
                alignItems="flex-start"
                sx={{
                    width: '100%',
                    backgroundColor: 'white',
                    paddingTop: '20px',
                    paddingBottom: '5px',
                    margin: '30px',
                    borderRadius: '15px',
                    borderColor: 'rgb(230, 228, 215)',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
                }}
            >
                <Typography
                    sx={{ 
                        fontSize: '1.7rem',
                        textAlign: 'left',
                        lineHeight: 1.5,
                        marginLeft: '20px',
                        marginBottom: '10px',
                    }}
                >
                    Projects
                </Typography>
                {userProjects?.map((project, index) => {
                    return (
                        <Box key={index} sx={{
                            width: '100%',
                            paddingRight: '20px', 
                            borderTopStyle: 'solid', 
                            borderTopWidth: 1,
                            transition: 'background-color 0.15s ease-in-out',
                            '&:hover': {
                                backgroundColor: 'rgba(0,0,0,0.08)', // or any theme color
                            },
                        }}>
                            <Link href={`/project/${project.id}`}>
                                <Box display={'flex'} flexDirection={'row'} sx={{width: '100%'}}>
                                    <Box display={'flex'} flexDirection={'column'} sx={{flex: 1, minWidth: 0, ml: '20px'}}>
                                        <Typography sx={{
                                            fontSize: '1.3rem',
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {project.name}
                                        </Typography>
                                        <Typography sx={{fontSize: '0.9rem', marginBottom: '10px'}}>Created: {formatTimestamp(project.datecreated)}</Typography>
                                        <Typography sx={{
                                            fontSize: '0.9rem', 
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {project.description}
                                        </Typography>
                                    </Box>
                                    <Box 
                                        display="flex" 
                                        flexDirection="row" 
                                        alignItems="flex-end"
                                        sx={{ gap: '15px' }}
                                    >
                                        <Box display="flex" flexDirection="column" alignItems="center" sx={{ minWidth: '80px' }}>
                                            <Typography 
                                                variant="h4" 
                                                sx={{ 
                                                    fontSize: textStyle.sectionTitleSize,
                                                    fontWeight: 'bold',
                                                    color: 'secondary.main'
                                                }}
                                            >
                                                0
                                            </Typography>
                                            <Typography 
                                                variant="body2" 
                                                sx={{ 
                                                    fontSize: textStyle.paragraphSize,
                                                    color: 'text.secondary'
                                                }}
                                            >
                                                Variants
                                            </Typography>
                                        </Box>
                                    </Box>
                                </Box>
                            </Link>
                        </Box>
                    )
                })}
            </Box>
        </Container>
    )
}