import { Container, Typography, Box, Button } from "@mui/material";
import AccessDenied from "@/app/components/AccessDenied";
import { getServerSession } from "next-auth";
import { authOptions } from "@/app/api/auth/[...nextauth]/route";
import formatTimestamp from "@/app/function/formatTimestamp";
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

async function getUserRecipeData(id) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT 
                    c.id,
                    c.name,
                    c.description,
                    c.userid,
                    c.created_at,
                    (
                        SELECT json_agg(
                            json_build_object(
                                'id', b.id,
                                'name', b.name,
                                'description', b.description,
                                'projectId', b.projectid,
                                'ownerId', b.ownerid,
                                'createdAt', b.created_at,
                                'position', cb.position
                            ) ORDER BY cb.position
                        )
                        FROM collection_branches cb
                        JOIN branches b ON cb.branchid = b.id
                        WHERE cb.collectionid = c.id
                    ) AS branches
                FROM collections c
                WHERE c.userid = %s
                ORDER BY c.created_at DESC;
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

export default async function({ params }) {
    const collection = await getUserRecipeData(params.userId)
    const session = await getServerSession(authOptions)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    if(!session || params.userId !== session.user.id.toString()) {
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
                    borderColor: 'rgb(230, 228, 215)',
                    borderStyle: 'solid',
                    borderWidth: 2,
                    boxShadow: '0px 4px 8px rgba(0, 0, 0, 0.1)'
                }}
            >
                <Box sx={{ flex: 1 }}>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.sectionTitleSize,
                            marginBottom: '0px',
                            textAlign: 'left'
                        }}
                    >
                        {collection[0]?.name || 'Collection Name'}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: '0.9rem',
                            marginBottom: '10px',
                            textAlign: 'left'
                        }}
                    >
                        Created: {collection.length && formatTimestamp(collection[0]?.created_at)}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.paragraphSize,
                            textAlign: 'left',
                            lineHeight: 1.5
                        }}
                    >
                        {collection[0]?.description || 'No description available'}
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
                            {collection[0]?.length || 0}
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
                        <Link href={`/`}>
                            <Button variant="contained">
                                Add Variant
                            </Button>
                        </Link>
                    </Box>
                </Box>
            </Box>
        </Container>
    )
}