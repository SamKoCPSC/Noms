export const revalidate = 10
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

async function getUserCollectionData(id) {
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
                    c.ownerid,
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
                WHERE c.ownerid = %s
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
    const collections = await getUserCollectionData(params.userId)
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
                            fontSize: textStyle.titleSize,
                            marginBottom: '0px',
                            textAlign: 'left'
                        }}
                    >
                        {"My Collections"}
                    </Typography>
                    {/* <Typography
                        sx={{ 
                            fontSize: '0.9rem',
                            marginBottom: '10px',
                            textAlign: 'left'
                        }}
                    >
                        Created: {collections.length && formatTimestamp(collections[0]?.created_at)}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.paragraphSize,
                            textAlign: 'left',
                            lineHeight: 1.5
                        }}
                    >
                        {collections[0]?.description || 'No description available'}
                    </Typography> */}
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
                            {collections.length || 0}
                        </Typography>
                        <Typography 
                            variant="body2" 
                            sx={{ 
                                fontSize: textStyle.paragraphSize,
                                color: 'text.secondary'
                            }}
                        >
                            Collections
                        </Typography>
                        <Link href={`/`}>
                            <Button variant="contained">
                                Add Collection
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
                    Collections
                </Typography>
                {collections?.map((collection) => {
                    return (
                        <Box key={collection.id} sx={{
                            width: '100%',
                            paddingRight: '20px', 
                            borderTopStyle: 'solid', 
                            borderTopWidth: 1,
                            transition: 'background-color 0.15s ease-in-out',
                            '&:hover': {
                                backgroundColor: 'rgba(0,0,0,0.08)', // or any theme color
                            },
                        }}>
                            <Link href={`/`}>
                                <Box display={'flex'} flexDirection={'row'} sx={{width: '100%'}}>
                                    {/* {collections && collections.length > 0 && (
                                        <Box 
                                            component="img"
                                            src={branch.recipes
                                                .sort((a, b) => new Date(b.datecreated) - new Date(a.datecreated))[0]
                                                ?.imageurls?.[0] || "/fallback.png"}
                                            alt={`${branch.name} preview`}
                                            sx={{
                                                width: '160px',
                                                height: '90px',
                                                objectFit: "cover",
                                                marginRight: '5px'
                                            }}
                                        />
                                    )} */}
                                    <Box display={'flex'} flexDirection={'column'} sx={{flex: 1, minWidth: 0, ml: '20px'}}>
                                        <Typography sx={{
                                            fontSize: '1.3rem',
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {collection.name}
                                        </Typography>
                                        <Typography sx={{fontSize: '0.9rem', marginBottom: '10px'}}>Created: {formatTimestamp(collection.created_at)}</Typography>
                                        <Typography sx={{
                                            fontSize: '0.9rem', 
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {collection.description}
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
                                                {collection.branches?.length || 0}
                                            </Typography>
                                            <Typography 
                                                variant="body2" 
                                                sx={{ 
                                                    fontSize: textStyle.paragraphSize,
                                                    color: 'text.secondary'
                                                }}
                                            >
                                                Recipes
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