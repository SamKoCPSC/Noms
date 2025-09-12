import { Typography, Container, Divider, Box, Avatar } from "@mui/material";
import RecipeCard from "@/app/components/RecipeCard";

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

export async function generateStaticParams() {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT DISTINCT branchbase, branchid
                FROM recipes
            `,
            values: []
        })
    }).then((response) => {
        if(!response.ok) {
            throw new Error(`HTTP error! Status: ${response.status}`)
        }
        return response.json()
    }).then((data) => {
        return data.result.map((branch) => ({
            branchbase: branch.branchbase.toString(),
            branchid: branch.branchid.toString(),
        }))
    })
    .catch((error) => {
        console.error(error)
        return []
    })
}

async function getBranchRecipes(branchid) {
    return fetch(process.env.LAMBDA_API_URL, {
        method: "POST",
        headers: {
            "Content-Type": "application/json",
            'x-api-key': process.env.LAMBDA_API_KEY,
        },
        body: JSON.stringify({
            sql: `
                SELECT 
                    b.id,
                    b.name,
                    b.description,
                    b.ownerid,
                    u.name AS ownername,
                    b.baserecipeid,
                    b.headrecipeid,
                    b.projectid,
                    b.created_at,
                    (
                        SELECT json_agg(
                            json_build_object(
                                'id', r.id,
                                'name', r.name,
                                'description', r.description,
                                'status', r.status,
                                'datecreated', r.datecreated,
                                'imageurls', r.imageurls,
                                'position', rb.position
                            ) ORDER BY rb.position
                        )
                        FROM recipe_branches rb
                        JOIN recipes r ON rb.recipeid = r.id
                        WHERE rb.branchid = b.id
                    ) AS recipes
                FROM branches b
                JOIN users u ON b.ownerid = u.id
                WHERE b.id = %s
                GROUP BY b.id, u.name;
            `,
            values: [branchid]
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

export default async function Recipe({ params }) {
    const variant = await getBranchRecipes(params.branchid)

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
                        {variant[0]?.name || 'Variation Name'}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: '0.9rem',
                            marginBottom: '10px',
                            textAlign: 'left'
                        }}
                    >
                        By: {variant[0]?.ownername || 'Variant Owner'} | Created: {formatTimestamp(variant[0]?.created_at)}
                    </Typography>
                    <Typography
                        sx={{ 
                            fontSize: textStyle.paragraphSize,
                            textAlign: 'left',
                            lineHeight: 1.5
                        }}
                    >
                        {variant[0]?.description || 'No description available'}
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
                            {variant[0]?.recipes.length || 0}
                        </Typography>
                        <Typography 
                            variant="body2" 
                            sx={{ 
                                fontSize: textStyle.paragraphSize,
                                color: 'text.secondary'
                            }}
                        >
                            Versions
                        </Typography>
                    </Box>
                </Box>
            </Box>
        </Container>
    )
}