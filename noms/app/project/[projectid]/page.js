//export const revalidate = 10
import { Typography, Container, Divider, Box } from "@mui/material";
import RecipeCard from "@/app/components/RecipeCard";
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
                SELECT DISTINCT baseid
                FROM recipes;
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((base) => {
            return base.baseid.toString()
        })
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

async function getProject(projectid) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
            SELECT 
                p.id,
                p.name,
                p.description,
                p.ownerid,
                u.name AS ownername,
                p.created_at,
                (
                    SELECT json_agg(
                        json_build_object(
                            'id', b.id,
                            'name', b.name,
                            'description', b.description,
                            'ownerid', b.ownerid,
                            'baserecipeid', b.baserecipeid,
                            'headrecipeid', b.headrecipeid,
                            'created_at', b.created_at,
                            'recipes', (
                                SELECT json_agg(
                                    json_build_object(
                                        'id', r.id,
                                        'name', r.name,
                                        'description', r.description,
                                        'status', r.status,
                                        'datecreated', r.datecreated,
                                        'imageurls', r.imageurls,
                                        'position', rb.position
                                    )
                                    ORDER by rb.position
                                )
                                FROM recipe_branches rb
                                JOIN recipes r ON rb.recipeid = r.id
                                WHERE rb.branchid = b.id
                            )
                        ) ORDER BY b.created_at
                    )
                    FROM branches b
                    WHERE b.projectid = p.id
                ) AS branches
            FROM projects p
            JOIN users u ON p.ownerid = u.id
            WHERE p.id = %s
            GROUP BY p.id, u.name;
            `,
            values: [projectid]
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
        return {message: error.message}
    })
}

export default async function Recipe({ params }) {
    const project = await getProject(params.projectid)

    const textStyle = {
        titleSize: '4.5rem',
        sectionTitleSize: '3.125rem',
        listItemSize: '2rem',
        paragraphSize: '1.25rem'
    }

    return (
        <Container sx={{justifyItems: 'center', width: '100%'}}>
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
                        {project[0]?.name || 'Project Name'}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: '0.9rem',
                            marginBottom: '10px',
                            textAlign: 'left'
                        }}
                    >
                        By: {project[0]?.ownername || 'Project Owner'} | Created: {formatTimestamp(project[0]?.created_at)}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.paragraphSize,
                            textAlign: 'left',
                            lineHeight: 1.5
                        }}
                    >
                        {project[0]?.description || 'No description available'}
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
                                color: 'primary.main'
                            }}
                        >
                            {project[0]?.branches?.length || 0}
                        </Typography>
                        <Typography 
                            variant="body2" 
                            sx={{ 
                                fontSize: textStyle.paragraphSize,
                                color: 'text.secondary'
                            }}
                        >
                            Branches
                        </Typography>
                    </Box>
                    <Box display="flex" flexDirection="column" alignItems="center" sx={{ minWidth: '80px' }}>
                        <Typography 
                            variant="h4" 
                            sx={{ 
                                fontSize: textStyle.sectionTitleSize,
                                fontWeight: 'bold',
                                color: 'secondary.main'
                            }}
                        >
                            {project[0]?.branches?.reduce((total, branch) => total + (branch.recipes?.length || 0), 0) || 0}
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
                    Branches
                </Typography>
                {project[0]?.branches.map((branch) => {
                    return (
                        <Box key={branch.id} sx={{
                            width: '100%',
                            paddingRight: '20px', 
                            borderTopStyle: 'solid', 
                            borderTopWidth: 1,
                            transition: 'background-color 0.15s ease-in-out',
                            '&:hover': {
                                backgroundColor: 'rgba(0,0,0,0.08)', // or any theme color
                            },
                        }}>
                            <Link href={`/branch/${branch.id}`}>
                                <Box display={'flex'} flexDirection={'row'} sx={{width: '100%'}}>
                                    {branch.recipes && branch.recipes.length > 0 && (
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
                                    )}
                                    <Box display={'flex'} flexDirection={'column'} sx={{flex: 1, minWidth: 0}}>
                                        <Typography sx={{
                                            fontSize: '1.3rem',
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {branch.name}
                                        </Typography>
                                        <Typography sx={{fontSize: '0.9rem', marginBottom: '10px'}}>Created: {formatTimestamp(branch.created_at)}</Typography>
                                        <Typography sx={{
                                            fontSize: '0.9rem', 
                                            textOverflow: 'ellipsis',
                                            overflow: 'hidden',
                                            whiteSpace: 'nowrap',
                                        }}>
                                            {branch.description}
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
                                                {branch.recipes?.length || 0}
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